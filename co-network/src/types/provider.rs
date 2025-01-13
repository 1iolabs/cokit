use crate::didcomm;
use libipld::DefaultParams;
use libp2p::{
	gossipsub, mdns, rendezvous,
	swarm::{NetworkBehaviour, SwarmEvent},
};
use libp2p_bitswap::{Bitswap, BitswapEvent};

/// Trait which can be implemented on NetworkBehaviours which provide gossipsub.
// pub trait HeadsBehaviourProvider {
// 	type Event;

// 	fn heads(&self) -> &heads::Behaviour;
// 	fn heads_mut(&mut self) -> &mut heads::Behaviour;

// 	/// Extract heads event from event.
// 	fn heads_event(event: &SwarmEvent<Self::Event>) -> Option<&heads::Event>;
// 	fn into_heads_event(event: SwarmEvent<Self::Event>) -> Result<heads::Event, SwarmEvent<Self::Event>>;

// 	fn handle_event<F: Fn(&heads::Event) -> bool>(
// 		event: SwarmEvent<Self::Event>,
// 		predicate: F,
// 	) -> Result<heads::Event, SwarmEvent<Self::Event>> {
// 		match Self::heads_event(&event) {
// 			Some(behaviour_event) if predicate(behaviour_event) => Self::into_heads_event(event),
// 			_ => Err(event),
// 		}
// 	}
// }

/// Trait which can be implemented on NetworkBehaviours which provide gossipsub.
pub trait GossipsubBehaviourProvider: NetworkBehaviour {
	fn gossipsub(&self) -> &gossipsub::Behaviour;
	fn gossipsub_mut(&mut self) -> &mut gossipsub::Behaviour;

	/// Extract gossipsub event from event.
	fn gossipsub_event(event: &<Self as NetworkBehaviour>::ToSwarm) -> Option<&gossipsub::Event>;
	fn into_gossipsub_event(
		event: <Self as NetworkBehaviour>::ToSwarm,
	) -> Result<gossipsub::Event, <Self as NetworkBehaviour>::ToSwarm>;

	fn handle_event<F: Fn(&gossipsub::Event) -> bool>(
		event: <Self as NetworkBehaviour>::ToSwarm,
		predicate: F,
	) -> Result<gossipsub::Event, <Self as NetworkBehaviour>::ToSwarm> {
		match Self::gossipsub_event(&event) {
			Some(behaviour_event) if predicate(behaviour_event) => Self::into_gossipsub_event(event),
			_ => Err(event),
		}
	}
}

/// Trait which can be implemented on NetworkBehaviours which provide bitswap.
pub trait BitswapBehaviourProvider: NetworkBehaviour {
	fn bitswap(&self) -> &Bitswap<DefaultParams>;
	fn bitswap_mut(&mut self) -> &mut Bitswap<DefaultParams>;

	/// Extract bitswap event from event.
	fn bitswap_event(event: &<Self as NetworkBehaviour>::ToSwarm) -> Option<&BitswapEvent>;
	fn into_bitswap_event(
		event: <Self as NetworkBehaviour>::ToSwarm,
	) -> Result<BitswapEvent, <Self as NetworkBehaviour>::ToSwarm>;

	fn handle_event<F: Fn(&BitswapEvent) -> bool>(
		event: <Self as NetworkBehaviour>::ToSwarm,
		predicate: F,
	) -> Result<BitswapEvent, <Self as NetworkBehaviour>::ToSwarm> {
		match Self::bitswap_event(&event) {
			Some(behaviour_event) if predicate(behaviour_event) => Self::into_bitswap_event(event),
			_ => Err(event),
		}
	}
}

/// Trait which can be implemented on NetworkBehaviours which provide didcomm.
pub trait DidcommBehaviourProvider: NetworkBehaviour {
	fn didcomm(&self) -> &didcomm::Behaviour;
	fn didcomm_mut(&mut self) -> &mut didcomm::Behaviour;

	/// Extract didcomm event from event.
	fn didcomm_event(event: &<Self as NetworkBehaviour>::ToSwarm) -> Option<&didcomm::Event>;
	fn into_didcomm_event(
		event: <Self as NetworkBehaviour>::ToSwarm,
	) -> Result<didcomm::Event, <Self as NetworkBehaviour>::ToSwarm>;

	fn clone_didcomm_event(event: &<Self as NetworkBehaviour>::ToSwarm) -> Option<didcomm::Event> {
		Self::didcomm_event(event).cloned()
	}

	fn swarm_didcomm_event(event: &SwarmEvent<<Self as NetworkBehaviour>::ToSwarm>) -> Option<&didcomm::Event> {
		if let SwarmEvent::Behaviour(event) = event {
			Self::didcomm_event(event)
		} else {
			None
		}
	}

	fn swarm_clone_didcomm_event(event: &SwarmEvent<<Self as NetworkBehaviour>::ToSwarm>) -> Option<didcomm::Event> {
		if let SwarmEvent::Behaviour(event) = event {
			Self::clone_didcomm_event(event)
		} else {
			None
		}
	}

	fn handle_event<F: Fn(&didcomm::Event) -> bool>(
		event: <Self as NetworkBehaviour>::ToSwarm,
		predicate: F,
	) -> Result<didcomm::Event, <Self as NetworkBehaviour>::ToSwarm> {
		match Self::didcomm_event(&event) {
			Some(behaviour_event) if predicate(behaviour_event) => Self::into_didcomm_event(event),
			_ => Err(event),
		}
	}
}

/// Trait which can be implemented on NetworkBehaviours which provide mDNS.
pub trait MdnsBehaviourProvider: NetworkBehaviour {
	fn mdns(&self) -> &mdns::tokio::Behaviour;
	fn mdns_mut(&mut self) -> &mut mdns::tokio::Behaviour;

	/// Extract mdns event from event.
	fn mdns_event(event: &<Self as NetworkBehaviour>::ToSwarm) -> Option<&mdns::Event>;
	fn into_mdns_event(
		event: <Self as NetworkBehaviour>::ToSwarm,
	) -> Result<mdns::Event, <Self as NetworkBehaviour>::ToSwarm>;

	fn swarm_mdns_event(event: &SwarmEvent<<Self as NetworkBehaviour>::ToSwarm>) -> Option<&mdns::Event> {
		if let SwarmEvent::Behaviour(event) = event {
			Self::mdns_event(event)
		} else {
			None
		}
	}

	fn handle_event<F: Fn(&mdns::Event) -> bool>(
		event: <Self as NetworkBehaviour>::ToSwarm,
		predicate: F,
	) -> Result<mdns::Event, <Self as NetworkBehaviour>::ToSwarm> {
		match Self::mdns_event(&event) {
			Some(behaviour_event) if predicate(behaviour_event) => Self::into_mdns_event(event),
			_ => Err(event),
		}
	}
}

/// Trait which can be implemented on NetworkBehaviours which provide Rendezvous client.
pub trait RendezvousClientBehaviourProvider: NetworkBehaviour {
	fn rendezvous_client(&self) -> &rendezvous::client::Behaviour;
	fn rendezvous_client_mut(&mut self) -> &mut rendezvous::client::Behaviour;

	/// Extract rendezvous::client event from event.
	fn rendezvous_client_event(event: &<Self as NetworkBehaviour>::ToSwarm) -> Option<&rendezvous::client::Event>;
	fn into_rendezvous_client_event(
		event: <Self as NetworkBehaviour>::ToSwarm,
	) -> Result<rendezvous::client::Event, <Self as NetworkBehaviour>::ToSwarm>;

	fn handle_event<F: Fn(&rendezvous::client::Event) -> bool>(
		event: <Self as NetworkBehaviour>::ToSwarm,
		predicate: F,
	) -> Result<rendezvous::client::Event, <Self as NetworkBehaviour>::ToSwarm> {
		match Self::rendezvous_client_event(&event) {
			Some(behaviour_event) if predicate(behaviour_event) => Self::into_rendezvous_client_event(event),
			_ => Err(event),
		}
	}
}
