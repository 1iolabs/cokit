use co_network::{Libp2pNetwork, Libp2pNetworkConfig};
use co_storage::BlockStorage;
use libipld::DefaultParams;
use libp2p::{identity::Keypair, PeerId};

pub struct Network {
	network: Libp2pNetwork,
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
		Self { network }
	}

	/// Access the network.
	pub fn network(&self) -> &Libp2pNetwork {
		&self.network
	}

	/// Convert to libp2p network.
	pub fn into_network(self) -> Libp2pNetwork {
		self.network
	}
}
