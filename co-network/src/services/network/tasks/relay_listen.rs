use crate::{
	network::{Behaviour, Context, NetworkEvent},
	types::network_task::NetworkTask,
};
use libp2p::{autonat, core::transport::ListenerId, swarm::SwarmEvent, Multiaddr, Swarm};
use multiaddr::Protocol;
use std::mem::take;

/// Try to listen to a relay when we are behind NAT.
#[derive(Debug)]
pub struct RelayListenTask {
	relay: Multiaddr,
	listener_id: Option<ListenerId>,
}
impl RelayListenTask {
	pub fn new(relay: Multiaddr) -> Self {
		Self { relay, listener_id: None }
	}
}
impl NetworkTask<Behaviour, Context> for RelayListenTask {
	fn execute(&mut self, swarm: &mut Swarm<Behaviour>, _context: &mut Context) {
		let listen = self.listener_id.is_some();
		let private = swarm.behaviour().autonat.nat_status() == autonat::NatStatus::Private;
		if !listen && private {
			self.listener_id = swarm.listen_on(self.relay.clone().with(Protocol::P2pCircuit)).ok();
		} else if listen && !private {
			if let Some(listener_id) = take(&mut self.listener_id) {
				swarm.remove_listener(listener_id);
			}
		}
	}

	fn on_swarm_event(
		&mut self,
		swarm: &mut Swarm<Behaviour>,
		context: &mut Context,
		event: SwarmEvent<NetworkEvent>,
	) -> Option<SwarmEvent<NetworkEvent>> {
		match &event {
			SwarmEvent::Behaviour(NetworkEvent::Autonat(autonat::Event::StatusChanged { old: _, new: _ })) => {
				self.execute(swarm, context);
			},
			_ => {},
		}
		Some(event)
	}

	fn is_complete(&mut self) -> bool {
		false
	}
}
