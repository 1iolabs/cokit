use crate::{
	library::settings_timeout::settings_timeout, services::network::DidCommSendNetworkTask, Action, CoContext,
	CO_ID_LOCAL,
};
use co_primitives::CoId;
use futures::{future::ready, stream, Stream, StreamExt};

/// Send DIDComm message to peer and respond with
pub fn didcomm_send(
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::DidCommSend { message_id, peer, message } => Some(
			stream::once(ready((context.clone(), message_id.clone(), *peer, message.clone())))
				.filter_map(move |(context, message_id, peer, message)| async move {
					let network = context.network_tasks().await?;
					let timeout = settings_timeout(&context, &CoId::from(CO_ID_LOCAL), Some("didcomm-send")).await;
					Some(match DidCommSendNetworkTask::send(network, [peer], message, timeout).await {
						Ok(peer) => Action::DidCommSent { message_id, peer, result: Ok(()) },
						Err(err) => Action::DidCommSent { message_id, peer, result: Err(err.into()) },
					})
				})
				.map(Ok),
		),
		_ => None,
	}
}
