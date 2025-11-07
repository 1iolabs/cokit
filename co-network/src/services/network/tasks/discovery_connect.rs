use crate::{
	discovery::{self, Discovery},
	network::{Behaviour, Context, NetworkEvent},
	types::network_task::NetworkTask,
};
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use libp2p::{swarm::SwarmEvent, PeerId, Swarm};
use std::collections::BTreeSet;

/// Connect peers using discovery.
#[derive(Debug)]
pub struct DiscoveryConnectNetworkTask {
	discovery: BTreeSet<Discovery>,
	connect_request: Option<u64>,
	sender: UnboundedSender<Result<BTreeSet<PeerId>, DiscoveryError>>,
	peers: BTreeSet<PeerId>,
}
impl DiscoveryConnectNetworkTask {
	pub fn new(discovery: BTreeSet<Discovery>) -> (Self, UnboundedReceiver<Result<BTreeSet<PeerId>, DiscoveryError>>) {
		let (tx, rx) = futures::channel::mpsc::unbounded();
		(Self { discovery, connect_request: None, sender: tx, peers: Default::default() }, rx)
	}
}
impl NetworkTask<Behaviour, Context> for DiscoveryConnectNetworkTask {
	fn execute(&mut self, swarm: &mut Swarm<Behaviour>, context: &mut Context) {
		match context.discovery.connect(swarm, self.discovery.clone()) {
			Ok(v) => {
				self.connect_request = Some(v);
			},
			Err(e) => {
				self.sender.unbounded_send(Err(e.into())).ok();
				self.sender.disconnect();
			},
		}
	}

	fn on_swarm_event(
		&mut self,
		_swarm: &mut Swarm<Behaviour>,
		_context: &mut Context,
		event: SwarmEvent<NetworkEvent>,
	) -> Option<SwarmEvent<NetworkEvent>> {
		// handle
		let send = match &event {
			SwarmEvent::Behaviour(NetworkEvent::Discovery(discovery_event)) => match discovery_event {
				discovery::Event::Connected { id, peer } if Some(*id) == self.connect_request => {
					self.peers.insert(*peer)
				},
				discovery::Event::Disconnected { id, peer } if Some(*id) == self.connect_request => {
					self.peers.remove(peer)
				},
				discovery::Event::Timeout { id } if Some(*id) == self.connect_request => {
					self.sender.unbounded_send(Err(DiscoveryError::Timeout)).ok();
					self.sender.disconnect();
					false
				},
				_ => false,
			},
			_ => false,
		};

		// send
		if send {
			match self.sender.unbounded_send(Ok(self.peers.clone())) {
				Ok(_) => {},
				Err(_) => {
					self.sender.disconnect();
				},
			}
		}

		// forward
		Some(event)
	}

	fn is_complete(&mut self) -> bool {
		self.sender.is_closed()
	}
}

/// Discovery has failed.
/// When receiving this error means the connect attempt (and its network task) has been stopped.
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
	#[error("Discovery connect failed")]
	Connect(#[from] discovery::ConnectError),

	#[error("Discovery connect timeout")]
	Timeout,
}
