pub mod publish;
pub mod tasks;

use self::tasks::did_discovery::{DidDiscoverySubscribe, DidDiscoveryUnsubscribe};
use co_identity::{IdentityResolver, PrivateIdentity, PrivateIdentityResolver};
use co_network::{
	discovery::Discovery, Behaviour, Context, FnOnceNetworkTask, Libp2pNetwork, Libp2pNetworkConfig,
	NetworkTaskSpawner, Shutdown,
};
use co_storage::BlockStorage;
use futures::{channel::oneshot, stream, Stream, StreamExt, TryStreamExt};
use libipld::DefaultParams;
use libp2p::{identity::Keypair, Multiaddr, PeerId};
use std::{collections::BTreeSet, future::ready, sync::Arc};
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
	pub fn new<S, I, P>(network_key: Keypair, storage: S, resolver: I, private_resolver: P) -> Self
	where
		S: BlockStorage<StoreParams = DefaultParams> + Send + Sync + 'static,
		I: IdentityResolver + Clone + Send + Sync + 'static,
		P: PrivateIdentityResolver + Clone + Send + Sync + 'static,
	{
		let network_peer_id = PeerId::from(network_key.public());
		let network_config = Libp2pNetworkConfig::from_keypair(network_key.clone());
		let network: Libp2pNetwork =
			Libp2pNetwork::new(network_config, storage, resolver, private_resolver).expect("network");
		tracing::info!(peer_id = ?network_peer_id, "network");
		Self { spawner: network.spawner(), peer_id: network_peer_id, network: Arc::new(Mutex::new(Some(network))) }
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
		if let Some(network) = self.network.lock().await.as_mut() {
			Some(network.shutdown())
		} else {
			None
		}
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

	pub async fn dail(&self, peer_id: PeerId, addresses: Vec<Multiaddr>) -> Result<(), anyhow::Error> {
		DialNetworkTask::dial(self.spawner(), peer_id, addresses).await
	}

	/// Connect networks and return the connect peers.
	pub fn connect(
		&self,
		networks: impl IntoIterator<Item = Discovery>,
	) -> impl Stream<Item = Result<PeerId, anyhow::Error>> {
		let (task, peers_stream) = DiscoveryConnectNetworkTask::new(networks.into_iter().collect());
		let spawner = self.spawner();
		async_stream::stream! {
			let mut known_peers = BTreeSet::<PeerId>::new();

			// execute
			match spawner.spawn(task) {
				Ok(_) => {},
				Err(e) => yield Err(e.into()),
			}

			// process
			for await peers in peers_stream {
				match peers {
					Ok(peers) => {
						for peer in peers {
							if !known_peers.contains(&peer) {
								known_peers.insert(peer);
								yield Ok(peer);
							}
						}
					},
					Err(e) => yield Err(e.into()),
				};
			}
		}
	}

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
				co_primitives::Network::DidDiscovery(item) => Some(item),
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
