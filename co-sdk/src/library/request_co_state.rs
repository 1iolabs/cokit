// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{library::wait_response::wait_response_timeout, services::application::ApplicationMessage, Action};
use anyhow::anyhow;
use co_actor::ActorHandle;
use co_identity::PrivateIdentity;
use co_network::{EncodedMessage, HeadsMessage, PeerId};
use co_primitives::{CoDateRef, CoId, WeakCid};
use futures::try_join;
use std::{collections::BTreeSet, time::Duration};

/// Request state (root + heads) from a peer.
pub async fn request_co_state<P>(
	actions: ActorHandle<ApplicationMessage>,
	co: &CoId,
	from: &P,
	to: PeerId,
	date: &CoDateRef,
	timeout: Duration,
) -> anyhow::Result<(WeakCid, BTreeSet<WeakCid>)>
where
	P: PrivateIdentity + Send + Sync + 'static,
{
	// request
	let body = HeadsMessage::StateRequest(co.clone());
	let header = HeadsMessage::create_header(date);
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
		HeadsMessage::State(received_co, state, heads) => {
			if &received_co != co {
				return Err(anyhow!("Received state for different CO"));
			}
			Ok((state, heads))
		},
		HeadsMessage::Error { co, code, message } => Err(anyhow!("Request failed ({:?}): {}: {}", code, co, message)),
		_ => Err(anyhow!("Unexpected response")),
	}
}
