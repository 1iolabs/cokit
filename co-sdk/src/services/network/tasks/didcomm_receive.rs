use co_identity::Message;
use co_network::{didcomm, DidcommBehaviourProvider, NetworkTask, NetworkTaskSpawner};
use futures::Stream;
use libp2p::{
	swarm::{NetworkBehaviour, SwarmEvent},
	PeerId, Swarm,
};
use std::fmt::Debug;

/// Handle received didcomm messages from network within the application.
pub struct DidCommReceiveNetworkTask {
	receive: tokio::sync::mpsc::UnboundedSender<(PeerId, Message)>,
}
impl Debug for DidCommReceiveNetworkTask {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("DidCommReceiveNetworkTask")
			.field("closed", &self.receive.is_closed())
			.finish()
	}
}
impl DidCommReceiveNetworkTask {
	pub fn receive<B, C, S>(spawner: S) -> impl Stream<Item = (PeerId, Message)> + Send + 'static
	where
		S: NetworkTaskSpawner<B, C> + Send + Sync + 'static,
		B: NetworkBehaviour + DidcommBehaviourProvider,
	{
		let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
		let task = Self { receive: tx };

		// spawn
		//  note: we intentionally ignoring the result as it only can be shutdown
		//   but in case of shutdown `tx` will be dropped and so `rx` will be closed and the stream is empty.
		spawner.spawn(task).ok();

		// result
		tokio_stream::wrappers::UnboundedReceiverStream::new(rx)
	}
}
impl<B, C> NetworkTask<B, C> for DidCommReceiveNetworkTask
where
	B: NetworkBehaviour + DidcommBehaviourProvider,
{
	fn execute(&mut self, _swarm: &mut Swarm<B>, _context: &mut C) {}

	fn on_swarm_event(
		&mut self,
		_swarm: &mut Swarm<B>,
		_context: &mut C,
		event: SwarmEvent<B::ToSwarm>,
	) -> Option<SwarmEvent<B::ToSwarm>> {
		if let Some(didcomm_event) = B::swarm_didcomm_event(&event) {
			if let didcomm::Event::Received { peer_id, message } = &didcomm_event {
				self.receive.send((*peer_id, message.clone())).ok();
			}
		}
		Some(event)
	}

	fn is_complete(&mut self) -> bool {
		self.receive.is_closed()
	}
}
