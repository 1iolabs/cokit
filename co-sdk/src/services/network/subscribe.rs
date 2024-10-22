use super::{
	tasks::did_discovery::{DidDiscoverySubscribe, DidDiscoveryUnsubscribe},
	CoNetworkTaskSpawner,
};
use co_identity::PrivateIdentity;
use co_network::NetworkTaskSpawner;
use co_primitives::{Did, NetworkDidDiscovery};
use futures::{stream, StreamExt, TryStreamExt};
use std::{fmt::Debug, future::ready};

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
	stream::iter(networks)
		.then(|network| async {
			let (task, result) = DidDiscoverySubscribe::new(identity.clone(), Some(network));
			spawner.spawn(task)?;
			result.await??;
			Ok::<(), anyhow::Error>(())
		})
		.try_for_each(|_| ready(Ok(())))
		.await?;

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
