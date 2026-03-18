// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{
	library::network_queue::TaskState,
	services::application::action::DidDidCommSendAction,
	Action, CoContext, CO_ID_LOCAL,
};
use co_actor::{Actions, ActorHandle};
use co_network::connections::ConnectionMessage;
use co_primitives::{BlockSerializer, CoId};
use futures::{future::Either, stream, FutureExt, Stream, StreamExt};
use std::collections::BTreeSet;

const NETWORK_QUEUE_TYPE: &str = "did-didcomm";

pub fn did_didcomm_send(
	actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::DidDidCommSend(message) => {
			let message = message.clone();
			let actions = actions.clone();
			let context = context.clone();
			Some(
				async move { context.network_connections().await }
					.into_stream()
					.flat_map(move |connections| {
						// network
						let Some(connections) = connections else {
							// this will queue the message for later
							return Either::Left(stream::iter([Ok(Action::DidDidCommSent {
								message: message.clone(),
								result: Ok(Default::default()),
							})]));
						};

						// send
						Either::Right(did_didcomm_send_message(connections, actions.clone(), message.clone()))
					}),
			)
		},
		_ => None,
	}
}

/// Queue when no peers could be found.
///
/// In: [`Action::DidDidCommSent`]
/// Out: [`Action::NetworkTaskQueue`]
pub fn did_didcomm_send_queue(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	_context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::DidDidCommSent { message, result: Ok(peers) }
			if !message.tags.contains_key("task_id") && peers.is_empty() =>
		{
			let co = message.co.clone().unwrap_or_else(|| CoId::from(CO_ID_LOCAL));
			let result = Action::network_task_queue(
				co,
				message.message_header.id.clone(),
				NETWORK_QUEUE_TYPE,
				format!("DIDComm {} to {}", message.message_header.id, &message.to),
				message,
			);
			Some(stream::iter([result]))
		},
		_ => None,
	}
}

/// Execute queued [`DidDidCommSendAction`].
///
/// In: [`Action::NetworkTaskExecute`]
/// Out: [`Action::DidDidCommSent`], [`Action::NetworkTaskExecuteComplete`]
pub fn network_task_execute(
	actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::NetworkTaskExecute { co, task_id, task_type, task } if task_type == NETWORK_QUEUE_TYPE => {
			let message = BlockSerializer::default().deserialize::<DidDidCommSendAction>(task);
			let context = context.clone();
			let task_id = task_id.clone();
			let co = co.clone();
			let actions = actions.clone();
			Some(
				async move {
					// deserialize
					let Ok(message) = message else {
						return Either::Left(stream::iter([Ok(Action::NetworkTaskExecuteComplete {
							co,
							task_id,
							task_state: TaskState::Failed,
						})]));
					};

					// network
					let Some(connections) = context.network_connections().await else {
						return Either::Left(stream::iter([Ok(Action::NetworkTaskExecuteComplete {
							co,
							task_id,
							task_state: TaskState::Backlog,
						})]));
					};

					// send
					Either::Right(
						did_didcomm_send_message(connections, actions, message).flat_map(move |item| match item {
							Ok(Action::DidDidCommSent { message, result }) => {
								let task_state = match &result {
									Ok(peers) if peers.is_empty() => TaskState::Backlog,
									Ok(_) => TaskState::Done,
									Err(_) => TaskState::Failed,
								};
								stream::iter(vec![
									Ok(Action::NetworkTaskExecuteComplete {
										co: co.clone(),
										task_id: task_id.clone(),
										task_state,
									}),
									Ok(Action::DidDidCommSent { message, result }),
								])
							},
							item => stream::iter(vec![item]),
						}),
					)
				}
				.into_stream()
				.flatten(),
			)
		},
		_ => None,
	}
}

fn did_didcomm_send_message(
	connections: ActorHandle<ConnectionMessage>,
	actions: Actions<Action, (), CoContext>,
	message: DidDidCommSendAction,
) -> impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static {
	async_stream::stream! {
		let peers_stream = ConnectionMessage::did_use(
			connections,
			message.message_from.clone(),
			message.to.clone(),
			message.networks.clone(),
		);
		let mut result = BTreeSet::new();
		for await peers in peers_stream {
			match peers {
				Ok(peers) => {
					for peer in peers.added {
						// register sent
						let send_message_id = message.message_header.id.clone();
						let sent_peer_fut = actions.clone().once_map(move |action| match action {
							Action::DidCommSent { message_header, peer, .. } if message_header.id == send_message_id => Some(*peer),
							_ => None,
						});

						// send
						yield Ok(Action::DidCommSend { message_header: message.message_header.clone(), peer, message: message.message.clone() });

						// wait sent
						let sent_peer = sent_peer_fut.await;
						match sent_peer {
							Ok(peer) => {
								// success
								result.insert(peer);
								break;
							},
							Err(err) => {
								tracing::warn!(?err, ?peer, "did-didcomm-send-failed");
							},
						}
					}
				},
				Err(err) => {
					tracing::warn!(?err, "did-didcomm-connect-failed");
				},
			}

			// has at least one peer?
			if !result.is_empty() {
				break;
			}
		}

		// result
		//  note: the set is empty when no peer could be connected
		yield Ok(Action::DidDidCommSent {
			message: message.clone(),
			result: Ok(result),
		})
	}
}
