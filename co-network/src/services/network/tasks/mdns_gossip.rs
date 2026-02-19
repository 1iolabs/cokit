// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{
	network::{Behaviour, Context, NetworkEvent},
	types::network_task::NetworkTask,
};
use libp2p::{mdns, swarm::SwarmEvent, Swarm};

/// Use discovered peers as gossip peers.
#[derive(Debug)]
pub struct MdnsGossipNetworkTask {}
impl MdnsGossipNetworkTask {
	pub fn new() -> Self {
		Self {}
	}
}
impl NetworkTask<Behaviour, Context> for MdnsGossipNetworkTask {
	fn execute(&mut self, _swarm: &mut Swarm<Behaviour>, _context: &mut Context) {}

	fn on_swarm_event(
		&mut self,
		swarm: &mut Swarm<Behaviour>,
		_context: &mut Context,
		event: SwarmEvent<NetworkEvent>,
	) -> Option<SwarmEvent<NetworkEvent>> {
		match &event {
			SwarmEvent::Behaviour(NetworkEvent::Mdns(mdns::Event::Discovered(list))) => {
				for (peer_id, _) in list {
					swarm.behaviour_mut().gossipsub.add_explicit_peer(peer_id);
				}
			},
			SwarmEvent::Behaviour(NetworkEvent::Mdns(mdns::Event::Expired(list))) => {
				for (peer_id, _) in list {
					swarm.behaviour_mut().gossipsub.remove_explicit_peer(peer_id);
				}
			},
			_ => {},
		}
		Some(event)
	}

	fn is_complete(&mut self) -> bool {
		false
	}
}
