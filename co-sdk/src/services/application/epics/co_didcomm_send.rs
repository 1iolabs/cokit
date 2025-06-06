use crate::{services::application::action::CoDidCommSendAction, Action, CoContext, ConnectionMessage};
use co_actor::{Actions, ActorHandle};
use futures::{future::Either, stream, FutureExt, Stream, StreamExt};
use std::collections::BTreeSet;

pub fn co_didcomm_send(
	actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::CoDidCommSend(message) => {
			let message = message.clone();
			let actions = actions.clone();
			let context = context.clone();
			Some(
				async move { context.network_connections().await }
					.into_stream()
					.flat_map(move |connections| {
						// network
						let Some(connections) = connections else {
							return Either::Left(stream::iter([Ok(Action::CoDidCommSent {
								message: message.clone(),
								result: Ok(Default::default()),
							})]));
						};

						// send
						Either::Right(co_didcomm_send_message(connections, actions.clone(), message.clone()))
					}),
			)
		},
		_ => None,
	}
}

fn co_didcomm_send_message(
	connections: ActorHandle<ConnectionMessage>,
	actions: Actions<Action, (), CoContext>,
	message: CoDidCommSendAction,
) -> impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static {
	async_stream::stream! {
		let peers_stream = ConnectionMessage::co_use(
			connections,
			message.co.clone(),
			message.message_from.clone(),
			message.networks.clone(),
		);
		let mut result = BTreeSet::new();
		for await peers in peers_stream {
			match peers {
				Ok(peers) => {
					for peer in peers.added {
						// send
						yield Ok(Action::DidCommSend { message_id: message.message_id.clone(), peer, message: message.message.clone() });

						// wait
						let send_message_id = message.message_id.clone();
						let sent_peer = actions.clone().once_map(move |action| match action {
							Action::DidCommSent { message_id, peer, .. } if message_id == &send_message_id => Some(*peer),
							_ => None,
						}).await;
						match sent_peer {
							Ok(peer) => {
								// success
								result.insert(peer);
								break;
							},
							Err(err) => {
								tracing::warn!(?err, ?peer, "co-didcomm-send-failed");
							},
						}
					}
				},
				Err(err) => {
					tracing::warn!(?err, "co-didcomm-connect-failed");
				},
			}
		}

		// result
		//  note: the set is empty when no peer could be connected
		yield Ok(Action::CoDidCommSent {
			message: message.clone(),
			result: Ok(result),
		})
	}
}
