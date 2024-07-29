use crate::drivers::network::CoNetworkTaskSpawner;
use co_identity::IdentityResolverBox;
use co_network::{
	discovery::{self, Discovery, DiscoveryBehaviour},
	DiscoveryLayerBehaviourProvider, NetworkTask, NetworkTaskSpawner,
};
use futures::{
	channel::mpsc::{UnboundedReceiver, UnboundedSender},
	Stream,
};
use libp2p::{
	swarm::{NetworkBehaviour, SwarmEvent},
	PeerId, Swarm,
};
use std::{collections::BTreeSet, time::Duration};
use tokio_stream::StreamExt;

/// Connect peers using discovery.
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

	/// Try to connect networks. Returns every peer which has been discovered.
	pub fn connect(
		spawner: CoNetworkTaskSpawner,
		networks: impl IntoIterator<Item = Discovery>,
	) -> impl Stream<Item = Result<PeerId, anyhow::Error>> + Send + 'static {
		let (task, peers_stream) = DiscoveryConnectNetworkTask::new(networks.into_iter().collect());
		async_stream::stream! {
			let mut known_peers = BTreeSet::<PeerId>::new();

			// execute
			match spawner.spawn(task) {
				Ok(_) => {},
				Err(e) => yield Err(e.into()),
			}

			// process
			for await peers in peers_stream {
				match peers {
					Ok(peers) => {
						for peer in peers {
							if !known_peers.contains(&peer) {
								known_peers.insert(peer);
								yield Ok(peer);
							}
						}
					},
					Err(e) => yield Err(e.into()),
				};
			}
		}
	}

	/// Try to connect networks. Returns every peer which has been discovered.
	/// Timeout is restarted after every returned peer.
	pub fn connect_with_timeout(
		spawner: CoNetworkTaskSpawner,
		networks: impl IntoIterator<Item = Discovery>,
		timeout: Duration,
	) -> impl Stream<Item = Result<PeerId, anyhow::Error>> + Send + 'static {
		Self::connect(spawner, networks).timeout(timeout).map(|item| match item {
			Ok(result) => result,
			Err(err) => Err(err.into()),
		})
	}
}
impl<B, C> NetworkTask<B, C> for DiscoveryConnectNetworkTask
where
	B: NetworkBehaviour + DiscoveryBehaviour,
	C: DiscoveryLayerBehaviourProvider<IdentityResolverBox, Event = <B as NetworkBehaviour>::ToSwarm>,
{
	fn execute(&mut self, swarm: &mut Swarm<B>, context: &mut C) {
		match context.discovery_mut().connect(swarm, self.discovery.clone()) {
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
		_swarm: &mut Swarm<B>,
		_context: &mut C,
		event: SwarmEvent<B::ToSwarm>,
	) -> Option<SwarmEvent<B::ToSwarm>> {
		// handle
		let send = match &event {
			SwarmEvent::Behaviour(behaviour_event) => match C::discovery_event(behaviour_event) {
				Some(discovery::Event::Connected { id, peer }) if Some(*id) == self.connect_request => {
					self.peers.insert(*peer)
				},
				Some(discovery::Event::Disconnected { id, peer }) if Some(*id) == self.connect_request => {
					self.peers.remove(peer)
				},
				Some(discovery::Event::Timeout { id }) if Some(*id) == self.connect_request => {
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
		!self.sender.is_closed()
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
