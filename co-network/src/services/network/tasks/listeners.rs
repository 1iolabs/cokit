use crate::CoNetworkTaskSpawner;
use co_network::{GossipsubBehaviourProvider, MdnsBehaviourProvider, NetworkTask, NetworkTaskSpawner};
use futures::channel::oneshot;
use libp2p::{
	swarm::{NetworkBehaviour, SwarmEvent},
	Multiaddr, Swarm,
};
use std::mem::take;

/// Get active listener addresses.
/// If no listener is present it will wait for the first to come available.
#[derive(Debug)]
pub struct ListnersNetworkTask {
	result: Option<oneshot::Sender<Vec<Multiaddr>>>,
}
impl ListnersNetworkTask {
	pub async fn listeners(spawner: &CoNetworkTaskSpawner) -> Result<Vec<Multiaddr>, anyhow::Error> {
		let (tx, rx) = oneshot::channel();
		spawner.spawn(ListnersNetworkTask { result: Some(tx) })?;
		Ok(rx.await?)
	}
}
impl<B, C> NetworkTask<B, C> for ListnersNetworkTask
where
	B: NetworkBehaviour + MdnsBehaviourProvider + GossipsubBehaviourProvider,
{
	fn execute(&mut self, _swarm: &mut Swarm<B>, _context: &mut C) {
		let listeners: Vec<Multiaddr> = _swarm.listeners().cloned().collect();
		if !listeners.is_empty() {
			if let Some(result) = take(&mut self.result) {
				result.send(listeners).ok();
			}
		}
	}

	fn on_swarm_event(
		&mut self,
		swarm: &mut Swarm<B>,
		context: &mut C,
		event: SwarmEvent<B::ToSwarm>,
	) -> Option<SwarmEvent<B::ToSwarm>> {
		match &event {
			SwarmEvent::NewListenAddr { listener_id: _, address: _ } => {
				self.execute(swarm, context);
			},
			_ => {},
		}
		Some(event)
	}

	fn is_complete(&mut self) -> bool {
		self.result.is_none()
	}
}
