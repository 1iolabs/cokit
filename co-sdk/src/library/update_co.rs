use crate::{
	library::wait_response::wait_response_timeout, services::application::ApplicationMessage, Action, CoReducer,
};
use anyhow::anyhow;
use cid::Cid;
use co_actor::ActorHandle;
use co_identity::PrivateIdentity;
use co_network::{EncodedMessage, HeadsMessage, PeerId};
use futures::try_join;
use std::time::Duration;

/// (Forcibily) request heads from peer and wait for response.
pub async fn update_co<P>(
	actions: ActorHandle<ApplicationMessage>,
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
	let header = HeadsMessage::create_header(co_reducer.date());
	let (message_header, message) = EncodedMessage::create_signed_json(from, header, &body)?;
	let ((_peer, message), _) = try_join!(
		wait_response_timeout(actions.clone(), timeout, {
			let message_id = message_header.id.clone();
			move |action| match action {
				Action::DidCommReceive { peer, message } if message.header().thid.as_ref() == Some(&message_id) => {
					Some((*peer, message.clone()))
				},
				_ => None,
			}
		}),
		async move {
			actions
				.dispatch(Action::DidCommSend { message_header, peer: to, message })
				.map_err(anyhow::Error::from)
		}
	)?;

	// response
	let heads_message: HeadsMessage = message.body_deserialize()?;
	match heads_message {
		HeadsMessage::Heads(received_co, received_heads) => {
			if &received_co != co_reducer.id() {
				return Err(anyhow!("Received heads for different CO"));
			}

			// note:
			//  the heads will be also merged by heads_message_heads epic
			//  whichever is faster but this makes sure that the heads are merged after this call
			co_reducer.join(received_heads.into_iter().map(Cid::from).collect()).await?;
		},
		HeadsMessage::Error { co, code, message } => {
			return Err(anyhow!("Request failed ({:?}): {}: {}", code, co, message));
		},
		_ => {},
	}

	// done
	Ok(())
}
