use crate::{
	library::{
		backoff::backoff_with_jitter,
		network_queue::{
			network_queue_action, network_queue_backlog, network_queue_message, network_queue_task_complete,
			network_queue_task_doing,
		},
	},
	network::PeersNetworkTask,
	Action, CoContext, CoUuid,
};
use co_actor::{Actions, Epic};
use co_identity::PrivateIdentity;
use co_primitives::{CoId, CoTryStreamExt};
use futures::{future::Either, stream, FutureExt, Stream, StreamExt};
use std::{collections::BTreeSet, future::ready};

/// If no peers could be found to send a DidComm message to a Co put it in the queue.
///
/// In: [`Action::CoDidCommSent`]
pub fn network_queue_message_epic(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::CoDidCommSent { message, result: Ok(peers) }
			if !message.tags.contains_key("task_id") && peers.is_empty() =>
		{
			let context = context.clone();
			let message = message.clone();
			Some(
				async move { network_queue_message(&context, message).await }
					.into_stream()
					.try_ignore_elements(),
			)
		},
		_ => None,
	}
}

/// When network has started try to process pending messages and listen to new discovered peers.
///
/// In: [`Action::NetworkStarted`]
/// Out: [`Action::NetworkQueueProcess`]
pub fn network_started_epic(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::NetworkStarted => {
			let context = context.clone();
			Some(
				async move {
					if let Some(network) = context.network_tasks().await {
						let initial = stream::once(ready(Ok(Action::NetworkQueueProcess { co: None, retry: 0 })));
						let peer_discovered = PeersNetworkTask::peers(&network)
							.map(|_peer_id| Ok(Action::NetworkQueueProcess { co: None, retry: 0 }));
						Either::Left(initial.chain(peer_discovered))
					} else {
						Either::Right(stream::empty())
					}
				}
				.into_stream()
				.flatten(),
			)
		},
		_ => None,
	}
}

/// Process board tasks.
///
/// In: [`Action::NetworkQueueProcess`]
/// Out: [`Action::NetworkQueueProcessComplete`], [`Action::NetworkQueueProcess`]
///
/// TODO: Concurrency.
/// TODO: On error clear task locks.
/// TODO: Add trigger when have mDNS discovery.
#[derive(Debug, Default)]
pub struct NetworkQueueProcessEpic {
	processing: bool,
	pending: Pending,
}
impl Epic<Action, (), CoContext> for NetworkQueueProcessEpic {
	fn epic(
		&mut self,
		actions: &Actions<Action, (), CoContext>,
		action: &Action,
		_state: &(),
		context: &CoContext,
	) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
		match action {
			Action::NetworkQueueProcess { co, retry } => {
				// already processing?
				if self.processing {
					self.pending.insert(co);
					return None;
				}
				self.processing = true;

				// process
				let process = process(actions, context, co);

				// complete
				let process_complete = process_complete(context, co, *retry);

				// result
				Some(Either::Left(process.chain(process_complete)))
			},
			Action::NetworkQueueProcessComplete { co, is_empty, retry } => {
				// remove pending when queue is empty
				if *is_empty {
					self.pending.remove(co);
				} else {
					self.pending.insert(co);
				}

				// clear processing
				self.processing = false;

				// retry
				if let Some(co) = self.pending.pop() {
					Some(Either::Right(
						{
							let retry = *retry + 1;
							async move {
								tokio::time::sleep(backoff_with_jitter(retry)).await;
								Ok(Action::NetworkQueueProcess { co, retry })
							}
						}
						.into_stream(),
					))
				} else {
					None
				}
			},
			_ => None,
		}
	}
}

#[derive(Debug, Default)]
enum Pending {
	#[default]
	None,
	All,
	Co(BTreeSet<CoId>),
}
impl Pending {
	/// Insert pending flag for all or use a single co.
	pub fn insert(&mut self, co: &Option<CoId>) {
		if let Some(co) = co {
			match self {
				Pending::None => {
					*self = Pending::Co([co.clone()].into());
				},
				Pending::All => {},
				Pending::Co(cos) => {
					cos.insert(co.clone());
				},
			}
		} else {
			match self {
				Pending::None | Pending::Co(_) => {
					*self = Pending::All;
				},
				Pending::All => {},
			}
		}
	}

	/// Remove pending flag for all or use a single co.
	pub fn remove(&mut self, co: &Option<CoId>) {
		if let Some(co) = co {
			match self {
				Pending::None => {},
				Pending::All => {},
				Pending::Co(cos) => {
					cos.remove(co);
					if cos.is_empty() {
						*self = Pending::None;
					}
				},
			}
		} else {
			match self {
				Pending::None => {},
				Pending::All | Pending::Co(_) => {
					*self = Pending::None;
				},
			}
		}
	}

	/// Pop next pending flag.
	pub fn pop(&mut self) -> Option<Option<CoId>> {
		match self {
			Pending::None => None,
			Pending::All => {
				*self = Pending::None;
				Some(None)
			},
			Pending::Co(cos) => {
				let result = if let Some(co) = cos.pop_first() { Some(Some(co)) } else { None };
				if cos.is_empty() {
					*self = Pending::None;
				}
				result
			},
		}
	}
}

fn process_complete(
	context: &CoContext,
	co: &Option<CoId>,
	retry: u32,
) -> impl Stream<Item = Result<Action, anyhow::Error>> {
	let co = co.clone();
	let context = context.clone();
	async move {
		let tasks = network_queue_backlog(context.clone(), {
			let co = co.clone();
			move |task| {
				if let Some(co) = &co {
					return task.tags.string("co") == Some(co.as_str());
				} else {
					true
				}
			}
		});
		let first = tasks.try_first().await?;
		Ok(Action::NetworkQueueProcessComplete { co, is_empty: first.is_none(), retry })
	}
	.into_stream()
}

fn process(
	actions: &Actions<Action, (), CoContext>,
	context: &CoContext,
	co: &Option<CoId>,
) -> impl Stream<Item = Result<Action, anyhow::Error>> {
	let co = co.clone();
	let actions = actions.clone();
	let context = context.clone();
	let lock = context.uuid().uuid();
	async_stream::try_stream! {
		let local_co = context.local_co_reducer().await?;
		let identity = context.local_identity().boxed();
		let tasks = network_queue_backlog(context.clone(), {
			let co = co.clone();
			move |task| {
				if let Some(co) = &co {
					return task.tags.string("co") == Some(co.as_str());
				} else {
					true
				}
			}
		});
		for await task in tasks {
			let task = task?;
			let (action, complete) = network_queue_action(&local_co, &task, &lock).await?;

			// doing
			if !network_queue_task_doing(&identity, &local_co, &task, &lock).await? {
				continue;
			}

			// register complete
			let complete_fut = actions.once_map(complete);

			// send
			yield action;

			// wait complete
			let task_state = complete_fut.await?;

			// move task
			network_queue_task_complete(&identity, &local_co, &task, &lock, task_state).await?;
		}
	}
}
