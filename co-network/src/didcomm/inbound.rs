// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use super::{EncodedMessage, Event};
use co_identity::{IdentityResolver, Message, PrivateIdentityResolver};
use libp2p::PeerId;

/// Handle inbound message.
pub async fn inbound_message<I, P>(
	identity_resolver: I,
	private_identity_resolver: P,
	peer_id: PeerId,
	encoded_message: EncodedMessage,
) -> Option<Event>
where
	I: IdentityResolver + Send + Sync + 'static,
	P: PrivateIdentityResolver + Send + Sync + 'static,
{
	match Message::receive(identity_resolver, private_identity_resolver, encoded_message.as_ref()).await {
		Ok(message) => Some(Event::Received { peer_id, message }),
		Err(err) => {
			tracing::warn!(?err, message = ?encoded_message, ?peer_id, "didcomm-receive-failure");
			Some(Event::InboundFailure { peer_id, error: err.to_string(), message: Some(encoded_message) })
		},
	}
}
