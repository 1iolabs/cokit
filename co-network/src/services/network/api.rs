use crate::{
	services::{
		connections::ConnectionMessage,
		heads::HeadsApi,
		network::{DialNetworkTask, ListnersNetworkTask, NetworkMessage},
	},
	CoNetworkTaskSpawner,
};
use co_actor::ActorHandle;
use multiaddr::{Multiaddr, PeerId};

#[derive(Debug, Clone)]
pub struct NetworkApi {
	pub(crate) _handle: ActorHandle<NetworkMessage>,
	pub(crate) connections: ActorHandle<ConnectionMessage>,
	pub(crate) heads: HeadsApi,
	pub(crate) spawner: CoNetworkTaskSpawner,
}
impl NetworkApi {
	pub fn connections(&self) -> &ActorHandle<ConnectionMessage> {
		&self.connections
	}

	pub fn heads(&self) -> &HeadsApi {
		&self.heads
	}

	pub fn spawner(&self) -> &CoNetworkTaskSpawner {
		&self.spawner
	}

	/// Get our local peer id.
	pub fn local_peer_id(&self) -> PeerId {
		self.spawner.local_peer_id()
	}

	/// Get active listener addresses.
	/// If no listener is present it will wait for the first to come available.
	pub async fn listeners(&self) -> Result<Vec<Multiaddr>, anyhow::Error> {
		ListnersNetworkTask::listeners(&self.spawner).await
	}

	/// Dial and wait for connection to be made or fail.
	pub async fn dial(&self, peer_id: Option<PeerId>, address: Vec<Multiaddr>) -> Result<PeerId, anyhow::Error> {
		// TODO: add to gossipsub?
		DialNetworkTask::dial(&self.spawner, peer_id, address).await
	}
}
