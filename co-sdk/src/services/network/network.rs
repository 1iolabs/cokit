use crate::network::{DialNetworkTask, ListnersNetworkTask};
use co_network::{Behaviour, Context, NetworkError, NetworkTaskBox, NetworkTaskSpawner, TokioNetworkTaskSpawner};
use libp2p::{Multiaddr, PeerId};

#[derive(Clone)]
pub struct CoNetworkTaskSpawner {
	pub(crate) spawner: TokioNetworkTaskSpawner<Behaviour, Context>,
	pub(crate) local_peer: PeerId,
}
impl CoNetworkTaskSpawner {
	pub fn local_peer_id(&self) -> PeerId {
		self.local_peer
	}

	pub async fn listeners(&self) -> Result<Vec<Multiaddr>, anyhow::Error> {
		ListnersNetworkTask::listeners(self).await
	}

	pub async fn dial(&self, peer_id: Option<PeerId>, address: Vec<Multiaddr>) -> Result<PeerId, anyhow::Error> {
		// TODO: add to gossipsub?
		DialNetworkTask::dial(self, peer_id, address).await
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
