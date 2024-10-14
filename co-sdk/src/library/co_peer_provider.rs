use super::{co_state::CoState, network_discovery::network_discovery};
use crate::{
	drivers::network::{tasks::discovery_connect::DiscoveryConnectNetworkTask, CoNetworkTaskSpawner},
	state, CoStorage,
};
use anyhow::anyhow;
use async_trait::async_trait;
use co_identity::{IdentityResolverBox, PrivateIdentity};
use co_network::{discovery, NetworkTaskSpawner, PeerProvider};
use co_primitives::OptionLink;
use futures::{Stream, StreamExt, TryStreamExt};
use libp2p::PeerId;
use std::{collections::BTreeSet, future::ready};

#[deprecated]
#[derive(Clone)]
pub struct CoPeerProvider<I> {
	state: CoState,
	storage: CoStorage,
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
		state: CoState,
	) -> Self {
		Self { storage, state, spawner, identity, identity_resolver }
	}
}
#[async_trait]
impl<I> PeerProvider for CoPeerProvider<I>
where
	I: PrivateIdentity + Clone + Send + Sync + 'static,
{
	fn peers(&self) -> impl Stream<Item = BTreeSet<PeerId>> + Send + 'static {
		self.try_peers().filter_map({
			move |result| {
				ready(match result {
					Ok(p) => Some(p),
					Err(err) => {
						tracing::warn!(?err, "co-peer-discovery-error");
						None
					},
				})
			}
		})
	}

	fn try_peers(&self) -> impl Stream<Item = Result<BTreeSet<PeerId>, anyhow::Error>> + Send + 'static {
		// task
		let spawner = self.spawner.clone();
		let identity_resolver = self.identity_resolver.clone();
		let identity = self.identity.clone();
		let state = self.state.clone();
		let storage = self.storage.clone();

		// stream
		async_stream::stream! {
			// storage/state
			let co_state = state.read_state().await;

			// discovery
			let discovery = match networks(&identity_resolver, &identity, &storage, co_state).await {
				Ok(value) => {
					if value.is_empty() {
						yield Err(anyhow!("No networks"));
						return;
					}
					value
				},
				Err(err) => {
					yield Err(err);
					return;
				},
			};
			let (task, peers) = DiscoveryConnectNetworkTask::new(discovery);

			// spawn
			match spawner.spawn(task) {
				Ok(_) => {},
				Err(err) => {
					yield Err(err.into());
					return;
				},
			}

			// yield
			for await peer in peers {
				match peer {
					Ok(value) => yield Ok(value),
					Err(err) => yield Err(err.into()),
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
	state: OptionLink<co_core_co::Co>,
) -> Result<BTreeSet<discovery::Discovery>, anyhow::Error>
where
	P: PrivateIdentity + Send + Sync + 'static,
{
	let networks = state::networks(storage, state).await?;
	let participants = state::participants(storage, state).await?.into_iter().map(|item| item.did);
	Ok(network_discovery(Some(identity_resolver), identity, networks, participants)
		.try_collect()
		.await?)
}
