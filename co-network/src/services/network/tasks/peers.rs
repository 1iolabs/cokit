// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{
	network::{Behaviour, Context, NetworkEvent},
	services::network::CoNetworkTaskSpawner,
	types::network_task::{NetworkTask, NetworkTaskSpawner},
};
use futures::Stream;
use libp2p::{swarm::SwarmEvent, PeerId, Swarm};
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
impl NetworkTask<Behaviour, Context> for PeersNetworkTask {
	fn execute(&mut self, _swarm: &mut Swarm<Behaviour>, _context: &mut Context) {}

	fn on_swarm_event(
		&mut self,
		_swarm: &mut Swarm<Behaviour>,
		_context: &mut Context,
		event: SwarmEvent<NetworkEvent>,
	) -> Option<SwarmEvent<NetworkEvent>> {
		#[cfg(feature = "native")]
		if let SwarmEvent::Behaviour(NetworkEvent::Mdns(libp2p::mdns::Event::Discovered(list))) = &event {
			for (peer_id, _) in list {
				self.tx.send(*peer_id).ok();
			}
		}
		Some(event)
	}

	fn is_complete(&mut self) -> bool {
		self.tx.is_closed()
	}
}
