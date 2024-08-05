use co_network::{GossipsubBehaviourProvider, MdnsBehaviourProvider, NetworkTask};
use libp2p::{
	mdns,
	swarm::{NetworkBehaviour, SwarmEvent},
	Swarm,
};

/// Use discovered peers as gossip peers.
pub struct MdnsGossipNetworkTask {}
impl MdnsGossipNetworkTask {
	pub fn new() -> Self {
		Self {}
	}
}
impl<B, C> NetworkTask<B, C> for MdnsGossipNetworkTask
where
	B: NetworkBehaviour + MdnsBehaviourProvider + GossipsubBehaviourProvider,
{
	fn execute(&mut self, _swarm: &mut Swarm<B>, _context: &mut C) {}

	fn on_swarm_event(
		&mut self,
		swarm: &mut Swarm<B>,
		_context: &mut C,
		event: SwarmEvent<B::ToSwarm>,
	) -> Option<SwarmEvent<B::ToSwarm>> {
		if let Some(mdns_event) = B::swarm_mdns_event(&event) {
			match mdns_event {
				mdns::Event::Discovered(list) => {
					for (peer_id, _) in list {
						swarm.behaviour_mut().gossipsub_mut().add_explicit_peer(peer_id);
					}
				},
				mdns::Event::Expired(list) => {
					for (peer_id, _) in list {
						swarm.behaviour_mut().gossipsub_mut().remove_explicit_peer(peer_id);
					}
				},
			}
		}
		Some(event)
	}

	fn is_complete(&mut self) -> bool {
		false
	}
}
