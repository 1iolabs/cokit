// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{
	library::network_queue::{
		network_queue_action, network_queue_backlog, network_queue_heads, network_queue_message, network_queue_task,
		network_queue_task_complete, network_queue_task_doing, TaskState,
	},
	services::application::{HeadsError, HeadsMessageReceivedAction},
	Action, CoContext, CoUuid,
};
use co_actor::{time, Actions, Epic};
use co_identity::PrivateIdentity;
use co_network::{backoff_with_jitter, HeadsMessage};
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
					.try_ignore_elements()
					.boxed(),
			)
		},
		Action::HeadsMessageComplete(
			message @ HeadsMessageReceivedAction { message: HeadsMessage::Heads(_co, _heads), .. },
			Err(HeadsError::Transient(_)),
		) => {
			let context = context.clone();
			let message = message.clone();
			Some(
				async move { network_queue_heads(&context, message).await }
					.into_stream()
					.try_ignore_elements()
					.boxed(),
			)
		},
		Action::NetworkTaskQueue { co, task_id, task_type, task_name, task } => {
			let context = context.clone();
			let co = co.clone();
			let task_id = task_id.clone();
			let task_type = task_type.clone();
			let task_name = task_name.clone();
			let task = task.clone();
			Some(
				async move { network_queue_task(&context, co, task_id, task_type, task_name, task).await }
					.into_stream()
					.try_ignore_elements()
					.boxed(),
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
		Action::NetworkStartComplete(Ok(())) => {
			let context = context.clone();
			Some(
				async move {
					if let Some(network) = context.network().await {
						let initial = stream::once(ready(Ok(Action::NetworkQueueProcess { co: None, retry: 0 })));
						let network_changed = network
							.network_changed()
							.map(|_| Ok(Action::NetworkQueueProcess { co: None, retry: 0 }));
						Either::Left(initial.chain(network_changed))
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
				self.pending.pop().map(|co| {
					Either::Right(
						{
							let retry = *retry + 1;
							async move {
								time::sleep(backoff_with_jitter(retry)).await;
								Ok(Action::NetworkQueueProcess { co, retry })
							}
						}
						.into_stream(),
					)
				})
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
				let result = cos.pop_first().map(Some);
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
					task.tags.string("co") == Some(co.as_str())
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
					task.tags.string("co") == Some(co.as_str())
				} else {
					true
				}
			}
		});
		for await task in tasks {
			let task = task?;
			let (action, complete) = match network_queue_action(&local_co, &task, &lock).await {
				Ok(result) => result,
				Err(err) => {
					// fail
					network_queue_task_complete(&identity, &local_co, &task, &lock, TaskState::Failed).await?;

					// log
					tracing::warn!(?task, ?err, "network-queue-task-failed");

					// skip
					continue;
				},
			};

			// doing
			if !network_queue_task_doing(&identity, &local_co, &task, &lock).await? {
				continue;
			}

			// register complete
			let complete_fut = actions.once_map(move |action| complete.is_complete(action));

			// send
			yield action;

			// wait complete
			let task_state = complete_fut.await?;

			// move task
			network_queue_task_complete(&identity, &local_co, &task, &lock, task_state).await?;
		}
	}
}
