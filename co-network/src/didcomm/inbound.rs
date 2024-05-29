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
		Err(e) => Some(Event::InboundFailure { peer_id, error: e.to_string(), message: Some(encoded_message) }),
	}
}
