pub mod heads;
pub mod subscribe;
pub mod tasks;

use self::tasks::did_discovery::{DidDiscoverySubscribe, DidDiscoveryUnsubscribe};
use co_identity::{IdentityResolver, PrivateIdentity};
use co_network::{Behaviour, Context, Libp2pNetwork, Libp2pNetworkConfig, NetworkTaskSpawner};
use co_storage::BlockStorage;
use futures::{stream, StreamExt, TryStreamExt};
use libipld::DefaultParams;
use libp2p::{identity::Keypair, PeerId};
use std::{future::ready, sync::Arc};
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
	pub fn new<S, R>(network_key: Keypair, storage: S, resolver: R) -> Self
	where
		S: BlockStorage<StoreParams = DefaultParams> + Send + Sync + 'static,
		R: IdentityResolver + Clone + Send + Sync + 'static,
	{
		let network_peer_id = PeerId::from(network_key.public());
		let network_config = Libp2pNetworkConfig::from_keypair(network_key.clone());
		let network: Libp2pNetwork = Libp2pNetwork::new(network_config, storage, resolver).expect("network");
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

	// /// Update heads of the CO with known peers.
	// /// One time operation.
	// /// Note: This will not wait for responses.
	// pub async fn update(&self, co_reducer: CoReducer) -> Result<(), anyhow::Error> {
	// 	let update = Publish::new(self.spawner(), co_reducer.id().clone(), co_reducer.mapping.clone(), true);
	// 	update.request(&co_reducer).await?;
	// 	Ok(())
	// }

	// /// Subscribe to CO changes.
	// pub async fn subscribe(&self, co_reducer: CoReducer) -> Result<Subscription, anyhow::Error> {
	// 	state::networks(&co_reducer.storage(), co_reducer.reducer_state().await.0)
	// 	Subscription::subscribe(self.spawner(), co_reducer).await
	// }

	/// Listen on identity requests (DID Discovery).
	pub async fn did_discovery_subscribe<I: PrivateIdentity + Clone + Send + Sync + 'static>(
		&self,
		identity: I,
	) -> Result<(), anyhow::Error> {
		// get did discovery networks
		let mut networks: Vec<_> = identity
			.networks()
			.into_iter()
			.filter_map(|network| match network {
				co_api::Network::DidDiscovery(item) => Some(item),
				_ => None,
			})
			.collect();
		if networks.is_empty() {
			networks.push(Default::default());
		}

		// subscribe
		//  by returning on any error happens in between
		let spwaner = self.spawner();
		stream::iter(networks)
			.then(|network| async {
				let (task, result) = DidDiscoverySubscribe::new(identity.clone(), network);
				spwaner.spawn(task)?;
				result.await??;
				Ok::<(), anyhow::Error>(())
			})
			.try_for_each(|_| ready(Ok(())))
			.await?;

		// result
		Ok(())
	}

	/// Listen on identity requests (DID Discovery).
	pub async fn did_discovery_unsubscribe<I: PrivateIdentity + Clone + Send + Sync + 'static>(
		&self,
		identity: I,
	) -> Result<(), anyhow::Error> {
		let (task, result) = DidDiscoveryUnsubscribe::new(identity.identity().to_owned());
		self.spawner().spawn(task)?;
		result.await??;
		Ok(())
	}
}

pub type CoNetworkTaskSpawner = NetworkTaskSpawner<Behaviour, Context>;
