use crate::{
	reactive::{context::ActionObservable, wait_response::wait_response},
	types::message::heads::HeadsMessage,
	Action, CoReducer,
};
use anyhow::anyhow;
use co_identity::PrivateIdentity;
use co_network::didcomm::EncodedMessage;
use futures::join;
use libp2p::PeerId;
use std::time::Duration;

/// (Forcibily) request heads from peer and wait for response.
pub async fn update_co<P>(
	actions: ActionObservable,
	co_reducer: &CoReducer,
	from: &P,
	to: PeerId,
	timeout: Duration,
) -> anyhow::Result<()>
where
	P: PrivateIdentity + Send + Sync + 'static,
{
	// request
	let body = HeadsMessage::HeadsRequest(co_reducer.id().clone());
	let header = HeadsMessage::create_header();
	let (message_id, message) = EncodedMessage::create_signed_json(from, header, &body)?;
	let (response, _) = join!(
		wait_response(actions.clone(), timeout, {
			let message_id = message_id.clone();
			move |action| match action {
				Action::DidCommReceive { peer, message } if message.header().thid.as_ref() == Some(&message_id) => {
					Some((*peer, message.clone()))
				},
				_ => None,
			}
		}),
		async move {
			actions.dispatch(Action::DidCommSend { message_id, peer: to, message });
		}
	);

	// response
	let (_peer, message) = response?;
	let heads_message: HeadsMessage = message.body_deserialize()?;
	match heads_message {
		HeadsMessage::Heads(received_co, received_heads) => {
			if &received_co != co_reducer.id() {
				return Err(anyhow!("Received heads fot different CO"));
			}
			// note:
			//  the heads will be also merged by heads_message_heads epic
			//  whichever is faster but this makes sure that the heads are merged after this call
			co_reducer.join(&received_heads).await?;
		},
		HeadsMessage::Error { code, message } => {
			return Err(anyhow!("Request failed: {:?}: {}", code, message));
		},
		_ => {},
	}

	// done
	Ok(())
}
