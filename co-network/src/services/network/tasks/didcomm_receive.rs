// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{
	didcomm,
	network::{Behaviour, Context, NetworkEvent},
	services::network::CoNetworkTaskSpawner,
	types::network_task::{NetworkTask, NetworkTaskSpawner},
};
use co_identity::Message;
use futures::Stream;
use libp2p::{swarm::SwarmEvent, PeerId, Swarm};
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
	pub fn receive(spawner: CoNetworkTaskSpawner) -> impl Stream<Item = (PeerId, Message)> + Send + 'static {
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
impl NetworkTask<Behaviour, Context> for DidCommReceiveNetworkTask {
	fn execute(&mut self, _swarm: &mut Swarm<Behaviour>, _context: &mut Context) {}

	fn on_swarm_event(
		&mut self,
		_swarm: &mut Swarm<Behaviour>,
		_context: &mut Context,
		event: SwarmEvent<NetworkEvent>,
	) -> Option<SwarmEvent<NetworkEvent>> {
		if let SwarmEvent::Behaviour(NetworkEvent::Didcomm(didcomm::Event::Received { peer_id, message })) = &event {
			self.receive.send((*peer_id, message.clone())).ok();
		}
		Some(event)
	}

	fn is_complete(&mut self) -> bool {
		self.receive.is_closed()
	}
}
