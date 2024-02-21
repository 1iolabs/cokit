use libipld::store::StoreParams;
use libp2p::swarm::SwarmEvent;
use libp2p_bitswap::{Bitswap, BitswapEvent};

/// Trait which can be implemented on NetworkBehaviours which provide bitswap.
pub trait BitswapBehaviourProvider {
	type StoreParams: StoreParams;
	type Event;

	fn bitswap(&self) -> &Bitswap<Self::StoreParams>;
	fn bitswap_mut(&mut self) -> &mut Bitswap<Self::StoreParams>;

	/// Extract bitswap event from event.
	fn bitswap_event(event: &SwarmEvent<Self::Event>) -> Option<&BitswapEvent>;
	fn into_bitswap_event(event: SwarmEvent<Self::Event>) -> Result<BitswapEvent, SwarmEvent<Self::Event>>;
}
