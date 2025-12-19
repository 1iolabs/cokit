use crate::{
	discovery::{self, Discovery},
	network::{Behaviour, Context, NetworkEvent},
	services::network::CoNetworkTaskSpawner,
	types::network_task::{NetworkTask, NetworkTaskSpawner},
};
use futures::Stream;
use libp2p::{swarm::SwarmEvent, Swarm};
use std::collections::BTreeSet;
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::wrappers::UnboundedReceiverStream;

/// Connect peers using discovery.
#[derive(Debug)]
pub struct DiscoveryConnectNetworkTask {
	discovery: BTreeSet<Discovery>,
	connect_request: Option<u64>,
	sender: UnboundedSender<Result<discovery::Event, discovery::ConnectError>>,
}
impl DiscoveryConnectNetworkTask {
	pub fn discover(
		spawner: CoNetworkTaskSpawner,
		discovery: BTreeSet<Discovery>,
	) -> impl Stream<Item = Result<discovery::Event, discovery::ConnectError>> {
		let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
		let task = Self { discovery, connect_request: None, sender: tx };
		spawner.spawn(task).ok();
		UnboundedReceiverStream::new(rx)
	}
}
impl NetworkTask<Behaviour, Context> for DiscoveryConnectNetworkTask {
	fn execute(&mut self, swarm: &mut Swarm<Behaviour>, context: &mut Context) {
		match context.discovery.connect(swarm, self.discovery.clone()) {
			Ok(v) => {
				self.connect_request = Some(v);
			},
			Err(e) => {
				self.sender.send(Err(e)).ok();
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
		let id_and_discovery_event = match &event {
			SwarmEvent::Behaviour(NetworkEvent::Discovery(discovery_event)) => match discovery_event {
				discovery::Event::Connected { id, .. } => Some((*id, discovery_event)),
				discovery::Event::Disconnected { id, .. } => Some((*id, discovery_event)),
				discovery::Event::InsufficentPeers { id } => Some((*id, discovery_event)),
				discovery::Event::Timeout { id } => Some((*id, discovery_event)),
			},
			_ => None,
		};

		// send
		match id_and_discovery_event {
			Some((id, discovery_event)) if Some(id) == self.connect_request => {
				self.sender.send(Ok(discovery_event.clone())).ok();
			},
			_ => {},
		}

		// forward
		Some(event)
	}

	fn is_complete(&mut self) -> bool {
		self.sender.is_closed()
	}
}
