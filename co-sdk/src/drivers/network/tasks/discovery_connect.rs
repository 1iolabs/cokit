use co_identity::IdentityResolverBox;
use co_network::{
	discovery::{self, Discovery, DiscoveryBehaviour},
	DiscoveryLayerBehaviourProvider, NetworkTask,
};
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use libp2p::{
	swarm::{NetworkBehaviour, SwarmEvent},
	PeerId, Swarm,
};
use std::collections::BTreeSet;

/// Connect peers using discovery.
pub struct DiscoveryConnectNetworkTask {
	discovery: BTreeSet<Discovery>,
	connect_request: Option<u64>,
	sender: UnboundedSender<Result<PeerId, DiscoveryError>>,
}
impl DiscoveryConnectNetworkTask {
	pub fn new(discovery: BTreeSet<Discovery>) -> (Self, UnboundedReceiver<Result<PeerId, DiscoveryError>>) {
		let (tx, rx) = futures::channel::mpsc::unbounded();
		(Self { discovery, connect_request: None, sender: tx }, rx)
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
		match &event {
			SwarmEvent::Behaviour(behaviour_event) => match C::discovery_event(behaviour_event) {
				Some(discovery::Event::Connected { id, peer }) if Some(*id) == self.connect_request => {
					match self.sender.unbounded_send(Ok(*peer)) {
						Ok(_) => {},
						Err(_) => {
							self.sender.disconnect();
						},
					}
				},
				Some(discovery::Event::Timeout { id }) if Some(*id) == self.connect_request => {
					self.sender.unbounded_send(Err(DiscoveryError::Timeout)).ok();
					self.sender.disconnect();
				},
				_ => {},
			},
			_ => {},
		}

		// forward
		Some(event)
	}

	fn is_complete(&mut self) -> bool {
		!self.sender.is_closed()
	}
}

#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
	#[error("Discovery connect failed")]
	Connect(#[from] discovery::ConnectError),

	#[error("Discovery connect timeout")]
	Timeout,
}
