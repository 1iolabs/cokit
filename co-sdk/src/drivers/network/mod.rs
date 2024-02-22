pub mod subscribe;

use self::subscribe::Subscription;
use crate::CoReducer;
use co_network::{Behaviour, Libp2pNetwork, Libp2pNetworkConfig, NetworkTaskSpawner};
use co_storage::BlockStorage;
use libipld::DefaultParams;
use libp2p::{identity::Keypair, PeerId};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct Network {
	network: Arc<Mutex<Option<Libp2pNetwork>>>,
	spawner: CoNetworkTaskSpawner,
}
impl Network {
	/// Create Network driver.
	///
	/// Todo:
	/// - Change keypair to Local CO
	///
	/// Panics:
	/// - Can not create the network.
	pub fn new<S>(network_key: Keypair, storage: S) -> Self
	where
		S: BlockStorage<StoreParams = DefaultParams> + Send + Sync + 'static,
	{
		let network_peer_id = PeerId::from(network_key.public());
		let network_config = Libp2pNetworkConfig::from_keypair(network_key.clone());
		let network: Libp2pNetwork = Libp2pNetwork::new(network_config, storage).expect("network");
		tracing::info!(peer_id = ?network_peer_id, "network");
		Self { spawner: network.spawner(), network: Arc::new(Mutex::new(Some(network))) }
	}

	/// Create network task spawner.
	pub fn spawner(&self) -> CoNetworkTaskSpawner {
		self.spawner.clone()
	}

	/// Convert to libp2p network.
	pub async fn into_network(self) -> Option<Libp2pNetwork> {
		self.network.lock().await.take()
	}

	/// Sync the CO.
	/// One time operation.
	pub async fn sync(co_reducer: CoReducer) -> Result<(), anyhow::Error> {}

	/// Subscribe to CO changes.
	pub async fn subscribe(&self, co_reducer: CoReducer) -> Result<Subscription, anyhow::Error> {
		Subscription::subscribe(self.spawner(), co_reducer).await
	}
}

pub type CoNetworkTaskSpawner = NetworkTaskSpawner<Behaviour>;
