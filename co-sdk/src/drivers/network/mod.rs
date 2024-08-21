pub mod publish;
pub mod subscribe;
pub mod tasks;
pub mod token;

use co_identity::{IdentityResolver, PrivateIdentity, PrivateIdentityResolver};
use co_network::{
	bitswap::StorageResolver, discovery::Discovery, Behaviour, Context, FnOnceNetworkTask, Libp2pNetwork,
	Libp2pNetworkConfig, NetworkError, NetworkTask, NetworkTaskSpawner, Shutdown, TokioNetworkTaskSpawner,
};
use co_storage::BlockStorage;
use futures::{channel::oneshot, Stream};
use libipld::DefaultParams;
use libp2p::{identity::Keypair, Multiaddr, PeerId};
use std::sync::Arc;
use subscribe::{subscribe_identity, unsubscribe_identity};
use tasks::{dial::DialNetworkTask, discovery_connect::DiscoveryConnectNetworkTask};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct Network {
	network: Arc<Mutex<Option<Libp2pNetwork>>>,
	spawner: CoNetworkTaskSpawner,
	peer_id: PeerId,
}
impl Network {
	/// Create Network driver.
	///
	/// Todo:
	/// - Change keypair to Local CO
	///
	/// Panics:
	/// - Can not create the network.
	pub fn new<S, I, P, T>(
		identifier: String,
		network_key: Keypair,
		storage_resolver: T,
		resolver: I,
		private_resolver: P,
	) -> Self
	where
		S: BlockStorage<StoreParams = DefaultParams> + Send + Sync + 'static,
		T: StorageResolver<S> + Send + Sync + 'static,
		I: IdentityResolver + Clone + Send + Sync + 'static,
		P: PrivateIdentityResolver + Clone + Send + Sync + 'static,
	{
		let network_peer_id = PeerId::from(network_key.public());
		let network_config = Libp2pNetworkConfig::from_keypair(network_key.clone());
		let network: Libp2pNetwork =
			Libp2pNetwork::new(identifier, network_config, storage_resolver, resolver, private_resolver)
				.expect("network");
		tracing::info!(peer_id = ?network_peer_id, "network");
		Self {
			spawner: CoNetworkTaskSpawner { spawner: network.spawner(), local_peer: network_peer_id },
			peer_id: network_peer_id,
			network: Arc::new(Mutex::new(Some(network))),
		}
	}

	/// Get local peer id.
	pub fn peer_id(&self) -> PeerId {
		self.peer_id
	}

	/// Get local listeners addresses.
	pub async fn listeners(&self) -> Result<Vec<Multiaddr>, anyhow::Error> {
		let (tx, rx) = oneshot::channel();
		self.spawner().spawn(FnOnceNetworkTask::new(|swarm, _| {
			tx.send(swarm.listeners().cloned().collect::<Vec<_>>()).unwrap();
		}))?;
		Ok(rx.await?)
	}

	/// Network shutdown token.
	pub async fn shutdown(&self) -> Option<Shutdown> {
		self.network.lock().await.as_mut().map(|network| network.shutdown())
	}

	/// Create network task spawner.
	pub fn spawner(&self) -> CoNetworkTaskSpawner {
		self.spawner.clone()
	}

	/// Convert to libp2p network.
	pub async fn into_network(self) -> Option<Libp2pNetwork> {
		self.network.lock().await.take()
	}

	/// Dail a peer directly.
	pub async fn dail(&self, peer_id: PeerId, addresses: Vec<Multiaddr>) -> Result<(), anyhow::Error> {
		DialNetworkTask::dial(self.spawner(), peer_id, addresses).await
	}

	/// Connect networks and return the connect peers.
	#[deprecated]
	pub fn connect(
		&self,
		networks: impl IntoIterator<Item = Discovery>,
	) -> impl Stream<Item = Result<PeerId, anyhow::Error>> {
		DiscoveryConnectNetworkTask::connect(self.spawner(), networks)
	}

	/// Listen on identity requests (DID Discovery).
	#[deprecated]
	pub async fn did_discovery_subscribe<I: PrivateIdentity + Clone + Send + Sync + 'static>(
		&self,
		identity: I,
	) -> Result<(), anyhow::Error> {
		let spawner = self.spawner();
		Ok(subscribe_identity(&spawner, &identity).await?)
	}

	/// Listen on identity requests (DID Discovery).
	#[deprecated]
	pub async fn did_discovery_unsubscribe<I: PrivateIdentity + Clone + Send + Sync + 'static>(
		&self,
		identity: I,
	) -> Result<(), anyhow::Error> {
		let spawner = self.spawner();
		Ok(unsubscribe_identity(&spawner, identity.identity().to_owned()).await?)
	}
}

pub struct CoNetworkTaskSpawner {
	spawner: TokioNetworkTaskSpawner<Behaviour, Context>,
	local_peer: PeerId,
}

impl Clone for CoNetworkTaskSpawner {
	fn clone(&self) -> Self {
		Self { spawner: self.spawner.clone(), local_peer: self.local_peer }
	}
}
impl CoNetworkTaskSpawner {
	pub fn local_peer_id(&self) -> PeerId {
		self.local_peer
	}
}
impl NetworkTaskSpawner<Behaviour, Context> for CoNetworkTaskSpawner {
	fn spawn<T>(&self, task: T) -> Result<(), NetworkError>
	where
		T: NetworkTask<Behaviour, Context> + Send + 'static,
	{
		self.spawner.spawn(task)
	}
}
