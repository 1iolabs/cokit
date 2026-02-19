// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{
	network::{Behaviour, Context, NetworkEvent},
	types::network_task::NetworkTask,
};
use libp2p::{
	identify,
	swarm::{dial_opts::DialOpts, SwarmEvent},
	Multiaddr, PeerId, Swarm,
};
use multiaddr::Protocol;

/// Dial all listen addresses when identifies a peer.
/// This supports establishing bidirectional connectivity.
#[derive(Debug)]
pub struct IdentifyDialNetworkTask {
	agent: String,
}
impl IdentifyDialNetworkTask {
	pub fn new(agent: String) -> Self {
		Self { agent }
	}
}
impl NetworkTask<Behaviour, Context> for IdentifyDialNetworkTask {
	fn execute(&mut self, _swarm: &mut Swarm<Behaviour>, _context: &mut Context) {}

	fn on_swarm_event(
		&mut self,
		swarm: &mut Swarm<Behaviour>,
		_context: &mut Context,
		event: SwarmEvent<NetworkEvent>,
	) -> Option<SwarmEvent<NetworkEvent>> {
		if let SwarmEvent::Behaviour(NetworkEvent::Identify(identify::Event::Received { info, .. })) = &event {
			if info.agent_version == self.agent {
				let peer_id: PeerId = info.public_key.clone().into();
				for addr in info.listen_addrs.iter() {
					if !is_private_ip(addr) {
						match swarm.dial(DialOpts::peer_id(peer_id).addresses(vec![addr.clone()]).build()) {
							Err(err) => {
								tracing::debug!(?err, ?peer_id, ?addr, "network-identify-dial-failed");
							},
							Ok(_) => {
								tracing::trace!(?peer_id, ?addr, "network-identify-dial");
							},
						}
					}
				}
			}
		}
		Some(event)
	}

	fn is_complete(&mut self) -> bool {
		false
	}
}

/// Check if a Multiaddr contains a private IP address
pub fn is_private_ip(multiaddr: &Multiaddr) -> bool {
	for component in multiaddr.into_iter() {
		match component {
			Protocol::Ip4(addr) => {
				return addr.is_private() ||    // 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
                       addr.is_loopback() ||   // 127.0.0.0/8
                       addr.is_link_local() || // 169.254.0.0/16
                       addr.is_unspecified(); // 0.0.0.0
			},
			Protocol::Ip6(addr) => {
				return addr.is_loopback() ||    // ::1
                       addr.is_unspecified() || // ::
                       // Unique Local Address (fc00::/7 where 8th bit is 1)
                       (addr.segments()[0] & 0xfe00 == 0xfc00) ||
                       // Link-Local unicast (fe80::/10)
                       (addr.segments()[0] & 0xffc0 == 0xfe80);
			},
			_ => continue,
		}
	}
	false
}
