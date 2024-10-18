pub mod publish;
pub mod subscribe;
pub mod tasks;
pub mod token;

use co_actor::ActorHandle;
use co_identity::{IdentityResolver, PrivateIdentity, PrivateIdentityResolver};
use co_network::{
	bitswap::BitswapMessage, Behaviour, Context, FnOnceNetworkTask, Libp2pNetwork, Libp2pNetworkConfig, NetworkError,
	NetworkTask, NetworkTaskSpawner, Shutdown, TokioNetworkTaskSpawner,
};
use futures::channel::oneshot;
use libipld::DefaultParams;
use libp2p::{identity::Keypair, Multiaddr, PeerId};
use std::sync::Arc;
use subscribe::{subscribe_identity, unsubscribe_identity};
use tasks::dial::DialNetworkTask;
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
	pub fn new<I, P>(
		identifier: String,
		network_key: Keypair,
		resolver: I,
		private_resolver: P,
		bitswap: ActorHandle<BitswapMessage<DefaultParams>>,
	) -> Self
	where
		I: IdentityResolver + Clone + Send + Sync + 'static,
		P: PrivateIdentityResolver + Clone + Send + Sync + 'static,
	{
		let network_peer_id = PeerId::from(network_key.public());
		let network_config = Libp2pNetworkConfig::from_keypair(network_key.clone());
		let network = Libp2pNetwork::new(identifier.clone(), network_config, resolver, private_resolver, bitswap)
			.expect("network");
		tracing::info!(application = &identifier, peer_id = ?network_peer_id, "network");
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

#[derive(Clone)]
pub struct CoNetworkTaskSpawner {
	spawner: TokioNetworkTaskSpawner<Behaviour, Context>,
	local_peer: PeerId,
}
impl CoNetworkTaskSpawner {
	pub fn local_peer_id(&self) -> PeerId {
		self.local_peer
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
	fn spawn<T>(&self, task: T) -> Result<(), NetworkError>
	where
		T: NetworkTask<Behaviour, Context> + Send + 'static,
	{
		self.spawner.spawn(task)
	}
}
