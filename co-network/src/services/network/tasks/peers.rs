use crate::{
	services::network::CoNetworkTaskSpawner,
	types::{
		network_task::{NetworkTask, NetworkTaskSpawner},
		provider::MdnsBehaviourProvider,
	},
};
use futures::Stream;
use libp2p::{
	mdns,
	swarm::{NetworkBehaviour, SwarmEvent},
	PeerId, Swarm,
};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

/// Notify about discovered peers.
#[derive(Debug)]
pub struct PeersNetworkTask {
	tx: mpsc::UnboundedSender<PeerId>,
}
impl PeersNetworkTask {
	pub fn peers(spawner: &CoNetworkTaskSpawner) -> impl Stream<Item = PeerId> + use<> + 'static {
		let (tx, rx) = mpsc::unbounded_channel();
		spawner.spawn(Self { tx }).ok();
		UnboundedReceiverStream::new(rx)
	}
}
impl<B, C> NetworkTask<B, C> for PeersNetworkTask
where
	B: NetworkBehaviour + MdnsBehaviourProvider,
{
	fn execute(&mut self, _swarm: &mut Swarm<B>, _context: &mut C) {}

	fn on_swarm_event(
		&mut self,
		_swarm: &mut Swarm<B>,
		_context: &mut C,
		event: SwarmEvent<B::ToSwarm>,
	) -> Option<SwarmEvent<B::ToSwarm>> {
		if let Some(mdns_event) = B::swarm_mdns_event(&event) {
			match mdns_event {
				mdns::Event::Discovered(list) => {
					for (peer_id, _) in list {
						self.tx.send(*peer_id).ok();
					}
				},
				_ => {},
			}
		}
		Some(event)
	}

	fn is_complete(&mut self) -> bool {
		self.tx.is_closed()
	}
}
