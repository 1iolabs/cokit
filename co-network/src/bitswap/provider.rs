use async_trait::async_trait;
use co_storage::StorageError;
use libipld::store::StoreParams;
use libp2p::{swarm::SwarmEvent, PeerId};
use libp2p_bitswap::{Bitswap, BitswapEvent};
use std::collections::BTreeSet;

pub trait BitswapBehaviourProvider {
	type StoreParams: StoreParams;
	type Event;

	fn bitswap(&self) -> &Bitswap<Self::StoreParams>;
	fn bitswap_mut(&mut self) -> &mut Bitswap<Self::StoreParams>;

	/// Extract bitswap event from event.
	fn bitswap_event(event: &SwarmEvent<Self::Event>) -> Option<&BitswapEvent>;
	fn into_bitswap_event(event: SwarmEvent<Self::Event>) -> Result<BitswapEvent, SwarmEvent<Self::Event>>;
}

#[async_trait]
pub trait PeerProvider {
	async fn peers(&self) -> Result<BTreeSet<PeerId>, StorageError>;
}
