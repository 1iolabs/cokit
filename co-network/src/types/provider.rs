use crate::didcomm;
use libipld::store::StoreParams;
use libp2p::{gossipsub, swarm::SwarmEvent};
use libp2p_bitswap::{Bitswap, BitswapEvent};

/// Trait which can be implemented on NetworkBehaviours which provide gossipsub.
pub trait GossipsubBehaviourProvider {
	type Event;

	fn gossipsub(&self) -> &gossipsub::Behaviour;
	fn gossipsub_mut(&mut self) -> &mut gossipsub::Behaviour;

	/// Extract gossipsub event from event.
	fn gossipsub_event(event: &SwarmEvent<Self::Event>) -> Option<&gossipsub::Event>;
	fn into_gossipsub_event(event: SwarmEvent<Self::Event>) -> Result<gossipsub::Event, SwarmEvent<Self::Event>>;

	fn handle_event<F: Fn(&gossipsub::Event) -> bool>(
		event: SwarmEvent<Self::Event>,
		predicate: F,
	) -> Result<gossipsub::Event, SwarmEvent<Self::Event>> {
		match Self::gossipsub_event(&event) {
			Some(behaviour_event) if predicate(behaviour_event) => Self::into_gossipsub_event(event),
			_ => Err(event),
		}
	}
}

/// Trait which can be implemented on NetworkBehaviours which provide bitswap.
pub trait BitswapBehaviourProvider {
	type StoreParams: StoreParams;
	type Event;

	fn bitswap(&self) -> &Bitswap<Self::StoreParams>;
	fn bitswap_mut(&mut self) -> &mut Bitswap<Self::StoreParams>;

	/// Extract bitswap event from event.
	fn bitswap_event(event: &SwarmEvent<Self::Event>) -> Option<&BitswapEvent>;
	fn into_bitswap_event(event: SwarmEvent<Self::Event>) -> Result<BitswapEvent, SwarmEvent<Self::Event>>;

	fn handle_event<F: Fn(&BitswapEvent) -> bool>(
		event: SwarmEvent<Self::Event>,
		predicate: F,
	) -> Result<BitswapEvent, SwarmEvent<Self::Event>> {
		match Self::bitswap_event(&event) {
			Some(behaviour_event) if predicate(behaviour_event) => Self::into_bitswap_event(event),
			_ => Err(event),
		}
	}
}

/// Trait which can be implemented on NetworkBehaviours which provide didcomm.
pub trait DidcommBehaviourProvider {
	type Event;

	fn didcomm(&self) -> &didcomm::Behaviour;
	fn didcomm_mut(&mut self) -> &mut didcomm::Behaviour;

	/// Extract didcomm event from event.
	fn didcomm_event(event: &SwarmEvent<Self::Event>) -> Option<&didcomm::Event>;
	fn into_didcomm_event(event: SwarmEvent<Self::Event>) -> Result<didcomm::Event, SwarmEvent<Self::Event>>;

	fn handle_event<F: Fn(&didcomm::Event) -> bool>(
		event: SwarmEvent<Self::Event>,
		predicate: F,
	) -> Result<didcomm::Event, SwarmEvent<Self::Event>> {
		match Self::didcomm_event(&event) {
			Some(behaviour_event) if predicate(behaviour_event) => Self::into_didcomm_event(event),
			_ => Err(event),
		}
	}
}
