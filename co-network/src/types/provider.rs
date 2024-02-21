use async_trait::async_trait;
use co_storage::StorageError;
use libp2p::{gossipsub, swarm::SwarmEvent, PeerId};
use std::collections::BTreeSet;

#[async_trait]
pub trait PeerProvider {
	async fn peers(&self) -> Result<BTreeSet<PeerId>, StorageError>;
}

/// Trait which can be implemented on NetworkBehaviours which provide gossipsub.
pub trait GossipsubBehaviourProvider {
	type Event;

	fn gossipsub(&self) -> &gossipsub::Behaviour;
	fn gossipsub_mut(&mut self) -> &mut gossipsub::Behaviour;

	/// Extract gossipsub event from event.
	fn gossipsub_event(event: &SwarmEvent<Self::Event>) -> Option<&gossipsub::Event>;
	fn into_gossipsub_event(event: SwarmEvent<Self::Event>) -> Result<gossipsub::Event, SwarmEvent<Self::Event>>;
}
