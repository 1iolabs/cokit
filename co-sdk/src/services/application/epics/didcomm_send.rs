use crate::{library::settings_timeout::settings_timeout, Action, ActionError, CoContext, CO_ID_LOCAL};
use co_actor::Actions;
use co_primitives::CoId;
use futures::{FutureExt, Stream};

/// Send DIDComm message to peer and respond with
pub fn didcomm_send(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::DidCommSend { message_header, peer, message } => Some({
			let context = context.clone();
			let message_header = message_header.clone();
			let peer = *peer;
			let message = message.clone();
			async move {
				let result = if let Some(network) = context.network().await {
					let timeout = settings_timeout(&context, &CoId::from(CO_ID_LOCAL), Some("didcomm-send")).await;
					network
						.didcomm_send([peer], message, timeout)
						.await
						.map_err(ActionError::from)
						.map(|_| ())
				} else {
					Err(anyhow::anyhow!("No network").into())
				};
				Ok(Action::DidCommSent { message_header, peer, result })
			}
			.into_stream()
		}),
		_ => None,
	}
}
