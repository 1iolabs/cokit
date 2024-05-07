use crate::NetworkTask;
use co_primitives::Network;
use libp2p::{
	swarm::{dial_opts::DialOpts, NetworkBehaviour, SwarmEvent},
	Multiaddr, PeerId, Swarm,
};
use std::{collections::BTreeSet, str::FromStr};

#[derive(Debug, Clone)]
pub struct PeerNetworkDiscovery {
	peer: PeerId,
	addresses: BTreeSet<Multiaddr>,
	state: NetworkDiscoveryState,
}
impl PeerNetworkDiscovery {
	pub fn from_network(network: &Network) -> Result<Option<Self>, anyhow::Error> {
		match network {
			Network::Peer { peer, addresses } => Ok(Some(Self {
				peer: PeerId::from_bytes(&peer)?,
				addresses: addresses
					.iter()
					.map(|address| Multiaddr::from_str(&address))
					.collect::<Result<BTreeSet<_>, _>>()?,
				state: NetworkDiscoveryState::None,
			})),
			_ => Ok(None),
		}
	}
}
impl<B> NetworkTask<B> for PeerNetworkDiscovery
where
	B: NetworkBehaviour,
{
	fn execute(&mut self, swarm: &mut Swarm<B>) {
		// already connected?
		if swarm.is_connected(&self.peer) {
			self.state = NetworkDiscoveryState::Connected;
			return;
		}

		// dail
		let opts = DialOpts::peer_id(self.peer.clone())
			.addresses(self.addresses.clone().into_iter().collect())
			.build();
		self.state = match swarm.dial(opts) {
			Ok(_) => NetworkDiscoveryState::Pending,
			Err(e) => NetworkDiscoveryState::Error(format!("{}", e)),
		}
	}

	fn on_swarm_event(
		&mut self,
		_swarm: &mut Swarm<B>,
		event: SwarmEvent<<B as NetworkBehaviour>::ToSwarm>,
	) -> Option<SwarmEvent<<B as NetworkBehaviour>::ToSwarm>> {
		match &event {
			SwarmEvent::ConnectionEstablished {
				peer_id,
				connection_id: _,
				endpoint: _,
				num_established: _,
				concurrent_dial_errors: _,
				established_in: _,
			} =>
				if peer_id == &self.peer {
					self.state = NetworkDiscoveryState::Connected;
				},
			SwarmEvent::OutgoingConnectionError { connection_id: _, peer_id, error } =>
				if peer_id == &Some(self.peer) {
					self.state = NetworkDiscoveryState::Error(format!("{}", error));
				},
			_ => {},
		}
		Some(event)
	}

	fn is_complete(&mut self) -> bool {
		match self.state {
			NetworkDiscoveryState::Connected | NetworkDiscoveryState::Error(_) => true,
			_ => false,
		}
	}
}

#[derive(Debug, Clone)]
enum NetworkDiscoveryState {
	None,
	Pending,
	Connected,
	Error(String),
}
