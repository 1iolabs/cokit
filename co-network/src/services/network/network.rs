use crate::{
	network::{Behaviour, Context},
	types::network_task::{NetworkTaskBox, NetworkTaskSpawner, TokioNetworkTaskSpawner},
	NetworkError,
};
use libp2p::PeerId;

#[derive(Clone)]
pub struct CoNetworkTaskSpawner {
	pub(crate) spawner: TokioNetworkTaskSpawner<Behaviour, Context>,
	pub(crate) local_peer: PeerId,
}
impl CoNetworkTaskSpawner {
	pub fn local_peer_id(&self) -> PeerId {
		self.local_peer
	}

	/// Create a closed network task spawner.
	/// For test use only.
	pub fn new_closed(local_peer: PeerId) -> Self {
		let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
		CoNetworkTaskSpawner { local_peer, spawner: TokioNetworkTaskSpawner { tasks: tx } }
	}
}
impl std::fmt::Debug for CoNetworkTaskSpawner {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("CoNetworkTaskSpawner")
			.field("local_peer", &self.local_peer)
			.finish()
	}
}
impl NetworkTaskSpawner<Behaviour, Context> for CoNetworkTaskSpawner {
	fn spawn_box(&self, task: NetworkTaskBox<Behaviour, Context>) -> Result<(), NetworkError> {
		self.spawner.spawn_box(task)
	}
}
