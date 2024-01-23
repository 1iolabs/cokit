use libp2p::{
	gossipsub::{Behaviour, ConfigBuilder, IdentTopic, MessageAuthenticity, PublishError, SubscriptionError},
	identity::Keypair,
};

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
pub fn publish(gossipsub: &mut Behaviour, rendezvous_point: IdentTopic, message: Vec<u8>) -> Result<(), Error> {
	tracing::info!(?rendezvous_point, "didcontact-publish");
	gossipsub
		.publish(rendezvous_point, message)
		.map(|_| ())
		.map_err(|e| -> Error { e.into() })
}

/// Subscribe InviteRequest's on gossipsub.
pub fn subscribe(gossipsub: &mut Behaviour, rendezvous_point: &IdentTopic) -> Result<bool, Error> {
	tracing::info!(?rendezvous_point, "didcontact-subscribe");
	gossipsub.subscribe(rendezvous_point).map_err(|e| -> Error { e.into() })
}

/// Unsubscribe InviteRequest's on gossipsub.
pub fn unsubscribe(gossipsub: &mut Behaviour, rendezvous_point: &IdentTopic) -> Result<bool, Error> {
	tracing::info!(?rendezvous_point, "didcontact-unsubscribe");
	gossipsub.unsubscribe(rendezvous_point).map_err(|e| -> Error { e.into() })
}

pub fn create_gossipsub(keypair: Keypair) -> Behaviour {
	let gossipsub_config = ConfigBuilder::default()
		.max_transmit_size(256 * 1024)
		.build()
		.expect("valid config");
	Behaviour::new(MessageAuthenticity::Signed(keypair), gossipsub_config).expect("Valid configuration")
}

impl From<PublishError> for Error {
	fn from(value: PublishError) -> Self {
		match value {
			PublishError::Duplicate => Self::Message,
			PublishError::SigningError(_) => Self::Message,
			PublishError::InsufficientPeers => Self::Network { source: value.into() },
			PublishError::MessageTooLarge => Self::Message,
			PublishError::TransformFailed(_) => Self::Message,
		}
	}
}

impl From<SubscriptionError> for Error {
	fn from(value: SubscriptionError) -> Self {
		match value {
			SubscriptionError::PublishError(e) => e.into(),
			SubscriptionError::NotAllowed => Error::Permission,
		}
	}
}
