use crate::{
	drivers::network::tasks::didcomm_send::DidCommSendNetworkTask,
	library::settings_timeout::settings_timeout,
	reactive::context::{ActionObservable, StateObservable},
	Action, CoContext, CO_ID_LOCAL,
};
use co_primitives::CoId;
use futures::{future::ready, Stream, StreamExt};

/// Send DIDComm message to peer and respond with
pub fn didcomm_send(
	actions: ActionObservable,
	_states: StateObservable,
	context: CoContext,
) -> impl Stream<Item = Action> + Send + 'static {
	actions
		.filter_map(|action| {
			ready(match action {
				Action::DidCommSend { message_id, peer, message } => Some((message_id, peer, message)),
				_ => None,
			})
		})
		.filter_map(move |(message_id, peer, message)| {
			let context = context.clone();
			async move {
				let network = context.network_tasks().await?;
				let timeout = settings_timeout(&context, &CoId::from(CO_ID_LOCAL), Some("didcomm-send")).await;
				Some(match DidCommSendNetworkTask::send(network, [peer], message, timeout).await {
					Ok(peer) => Action::DidCommSent { message_id, peer, result: Ok(()) },
					Err(err) => Action::DidCommSent { message_id, peer, result: Err(err.into()) },
				})
			}
		})
}
