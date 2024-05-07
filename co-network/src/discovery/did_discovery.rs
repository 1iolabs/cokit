use crate::{heads, HeadsBehaviourProvider, NetworkTask};
use co_identity::{Identity, PrivateIdentity};
use libp2p::{
	swarm::{NetworkBehaviour, SwarmEvent},
	PeerId, Swarm,
};
use std::{
	collections::BTreeSet,
	time::{Duration, Instant},
};

#[derive(Debug, Clone)]
struct DidDiscoveryNetworkDiscovery<F, T>
where
	F: PrivateIdentity + Send + Sync + 'static,
	T: Identity + Send + Sync + 'static,
{
	topic: String,
	from: F,
	to: T,
	timeout: Duration,
	state: NetworkDiscoveryState,
}
impl<F, T> DidDiscoveryNetworkDiscovery<F, T>
where
	F: PrivateIdentity + Send + Sync + 'static,
	T: Identity + Send + Sync + 'static,
{
	pub fn new(topic: String, timeout: Duration, from: F, to: T) -> Self {
		Self { topic, from, to, timeout, state: NetworkDiscoveryState::None }
	}

	fn is_response(&self, message: &crate::didcomm::Message) -> bool {
		// TODO: implement
		false
	}
}
impl<B, F, T> NetworkTask<B> for DidDiscoveryNetworkDiscovery<F, T>
where
	B: NetworkBehaviour + HeadsBehaviourProvider<Event = <B as NetworkBehaviour>::ToSwarm>,
	F: PrivateIdentity + Send + Sync + 'static,
	T: Identity + Send + Sync + 'static,
{
	fn execute(&mut self, swarm: &mut Swarm<B>) {
		let heads = swarm.behaviour_mut().heads_mut();
		self.state = match heads.did_discover(&self.topic, &self.from, &self.to, "co/invite".to_string()) {
			Ok(true) => NetworkDiscoveryState::Pending(Instant::now()),
			Ok(false) => NetworkDiscoveryState::Connected(Default::default()),
			Err(e) => NetworkDiscoveryState::Error(format!("{}", e)),
		};
	}

	fn on_swarm_event(
		&mut self,
		_swarm: &mut Swarm<B>,
		event: SwarmEvent<<B as NetworkBehaviour>::ToSwarm>,
	) -> Option<SwarmEvent<<B as NetworkBehaviour>::ToSwarm>> {
		match B::heads_event(&event) {
			Some(heads::Event::Didcomm(crate::didcomm::Event::Received { peer_id, message })) => {
				if self.is_response(message) {
					self.state.insert_connected_peer(peer_id.clone());
				}
			},
			_ => {},
		}
		Some(event)
	}

	fn is_complete(&mut self) -> bool {
		match self.state {
			NetworkDiscoveryState::Connected(_) | NetworkDiscoveryState::Error(_) => true,
			NetworkDiscoveryState::Pending(start) =>
				if start.elapsed() > self.timeout {
					self.state = NetworkDiscoveryState::Timeout;
					true
				} else {
					false
				},
			_ => false,
		}
	}
}

#[derive(Debug, Clone)]
enum NetworkDiscoveryState {
	None,
	Pending(Instant),
	Connected(BTreeSet<PeerId>),
	Error(String),
	Timeout,
}
impl NetworkDiscoveryState {
	pub fn insert_connected_peer(&mut self, peer: PeerId) {
		match self {
			NetworkDiscoveryState::Connected(v) => {
				v.insert(peer);
			},
			_ => {
				let mut v = BTreeSet::new();
				v.insert(peer);
				*self = NetworkDiscoveryState::Connected(v)
			},
		}
	}
}
