use super::{co_state::CoState, network_discovery::network_discovery};
use crate::{
	drivers::network::{tasks::discovery_connect::DiscoveryConnectNetworkTask, CoNetworkTaskSpawner},
	state, CoStorage,
};
use async_trait::async_trait;
use co_identity::{IdentityResolverBox, PrivateIdentity};
use co_network::{discovery, NetworkTaskSpawner, PeerProvider};
use co_primitives::{CoId, OptionLink};
use futures::{Stream, TryStreamExt};
use libp2p::PeerId;
use std::collections::BTreeSet;

pub struct CoPeerProvider<I> {
	storage: CoStorage,
	id: CoId,
	state: CoState,
	identity_resolver: IdentityResolverBox,
	identity: I,
	spawner: CoNetworkTaskSpawner,
}
impl<I> CoPeerProvider<I>
where
	I: PrivateIdentity + Clone + Send + Sync + 'static,
{
	pub fn new(
		spawner: CoNetworkTaskSpawner,
		identity_resolver: IdentityResolverBox,
		identity: I,
		storage: CoStorage,
		id: CoId,
		state: CoState,
	) -> Self {
		Self { storage, id, state, spawner, identity, identity_resolver }
	}
}
#[async_trait]
impl<I> PeerProvider for CoPeerProvider<I>
where
	I: PrivateIdentity + Clone + Send + Sync + 'static,
{
	fn peers(&self) -> impl Stream<Item = BTreeSet<PeerId>> + Send + 'static {
		// task
		let spawner = self.spawner.clone();
		let identity_resolver = self.identity_resolver.clone();
		let identity = self.identity.clone();
		let storage = self.storage.clone();
		let state = self.state.clone();
		let id = self.id.clone();

		// stream
		async_stream::stream! {
			let discovery = match networks(&identity_resolver, &identity, &storage, &id, state.read().await).await {
				Ok(value) => value,
				Err(_) => return,
			};
			let (task, peers) = DiscoveryConnectNetworkTask::new(discovery);

			// spawn
			if spawner.spawn(task).is_err() {
				return
			}

			// yield
			for await peer in peers {
				match peer {
					Ok(value) => yield value,
					Err(_) => return
				}
			}
		}
	}
}

/// Create Discovery items from co and participant networks.
async fn networks<P>(
	identity_resolver: &IdentityResolverBox,
	identity: &P,
	storage: &CoStorage,
	id: &CoId,
	state: OptionLink<co_core_co::Co>,
) -> Result<BTreeSet<discovery::Discovery>, anyhow::Error>
where
	P: PrivateIdentity + Send + Sync + 'static,
{
	let networks = state::networks(storage, state).await?;
	let participants = state::participants(storage, state).await?.into_iter().map(|item| item.did);
	Ok(network_discovery(Some(identity_resolver), identity, Some(id), networks, participants)
		.try_collect()
		.await?)
}
