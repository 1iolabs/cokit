use super::{
	tasks::did_discovery::{DidDiscoverySubscribe, DidDiscoveryUnsubscribe},
	CoNetworkTaskSpawner,
};
use co_identity::PrivateIdentity;
use co_network::NetworkTaskSpawner;
use co_primitives::{Did, NetworkDidDiscovery};
use std::fmt::Debug;

/// Listen on identity requests (DID Discovery).
pub async fn subscribe_identity<I: PrivateIdentity + Debug + Clone + Send + Sync + 'static>(
	spawner: &CoNetworkTaskSpawner,
	identity: &I,
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
		networks.push(NetworkDidDiscovery { did: identity.identity().to_owned(), topic: Default::default() });
	}

	// subscribe
	//  by returning on any error happens in between
	for network in networks {
		let (task, result) = DidDiscoverySubscribe::new(identity.clone(), Some(network));
		spawner.spawn(task)?;
		result.await??;
	}

	// result
	Ok(())
}

/// Listen on identity requests (DID Discovery).
pub async fn unsubscribe_identity(spawner: &CoNetworkTaskSpawner, did: Did) -> Result<(), anyhow::Error> {
	let (task, result) = DidDiscoveryUnsubscribe::new(did);
	spawner.spawn(task)?;
	result.await??;
	Ok(())
}
