use libp2p::{gossipsub, identity::Keypair};

// modules
mod did;
mod rendezvouz_point;

// re-exports
pub use did::{resolve, ResolveError, ResolveResult};
pub use rendezvouz_point::RendezvousPoint;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	/// A Error with the message.
	#[error("A Error with the message.")]
	Message,

	/// A Error with the Network. Should be retriable when network conditions change.
	#[error("A Error with the Network.")]
	Network {
		#[source]
		source: anyhow::Error,
	},

	/// A Error due to insuficient permissions.
	#[error("A Error due to insuficient permissions.")]
	Permission,
}

/// Publish InviteRequest on gossipsub.
pub fn publish(
	gossipsub: &mut gossipsub::Behaviour,
	rendezvous_point: gossipsub::IdentTopic,
	message: Vec<u8>,
) -> Result<(), Error> {
	tracing::info!(?rendezvous_point, "didcontact-publish");
	gossipsub
		.publish(rendezvous_point, message)
		.map(|_| ())
		.map_err(|e| -> Error { e.into() })
}

/// Subscribe InviteRequest's on gossipsub.
pub fn subscribe(
	gossipsub: &mut gossipsub::Behaviour,
	rendezvous_point: &gossipsub::IdentTopic,
) -> Result<bool, Error> {
	tracing::info!(?rendezvous_point, "didcontact-subscribe");
	gossipsub.subscribe(rendezvous_point).map_err(|e| -> Error { e.into() })
}

/// Unsubscribe InviteRequest's on gossipsub.
pub fn unsubscribe(
	gossipsub: &mut gossipsub::Behaviour,
	rendezvous_point: &gossipsub::IdentTopic,
) -> Result<bool, Error> {
	tracing::info!(?rendezvous_point, "didcontact-unsubscribe");
	gossipsub.unsubscribe(rendezvous_point).map_err(|e| -> Error { e.into() })
}

pub fn create_gossipsub(keypair: Keypair) -> gossipsub::Behaviour {
	let gossipsub_config = gossipsub::ConfigBuilder::default()
		.max_transmit_size(256 * 1024)
		.build()
		.expect("valid config");
	gossipsub::Behaviour::new(gossipsub::MessageAuthenticity::Signed(keypair), gossipsub_config)
		.expect("Valid configuration")
}

impl From<gossipsub::PublishError> for Error {
	fn from(value: gossipsub::PublishError) -> Self {
		match value {
			gossipsub::PublishError::Duplicate => Self::Message,
			gossipsub::PublishError::SigningError(_) => Self::Message,
			gossipsub::PublishError::InsufficientPeers => Self::Network { source: value.into() },
			gossipsub::PublishError::MessageTooLarge => Self::Message,
			gossipsub::PublishError::TransformFailed(_) => Self::Message,
		}
	}
}

impl From<gossipsub::SubscriptionError> for Error {
	fn from(value: gossipsub::SubscriptionError) -> Self {
		match value {
			gossipsub::SubscriptionError::PublishError(e) => e.into(),
			gossipsub::SubscriptionError::NotAllowed => Error::Permission,
		}
	}
}
