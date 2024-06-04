use super::co_state::CoState;
use crate::{
	drivers::network::{tasks::discovery_connect::DiscoveryConnectNetworkTask, CoNetworkTaskSpawner},
	state, CoStorage,
};
use async_trait::async_trait;
use co_identity::{IdentityResolverBox, PrivateIdentity};
use co_network::{
	discovery::{self, Discovery},
	heads::HeadsState,
	DidDiscoveryMessage, PeerProvider,
};
use co_primitives::{CoId, Network, OptionLink};
use futures::Stream;
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
) -> Result<BTreeSet<Discovery>, anyhow::Error>
where
	P: PrivateIdentity + Send + Sync + 'static,
{
	let co_networks = state::networks(&storage, state)
		.await?
		.into_iter()
		.filter_map(|network| match network {
			Network::CoHeads(value) => Some(Discovery::Topic(HeadsState::to_topic_hash(&value, &id).into_string())),
			Network::Rendezvous(value) => Some(Discovery::Rendezvous(value)),
			Network::Peer(value) => Some(Discovery::Peer(value)),
			_ => None,
		});
	let participant_networks = state::participant_identities(identity_resolver, storage, state)
		.await?
		.into_iter()
		.flat_map(|participant| {
			identity.networks().into_iter().filter_map(move |network| match network {
				Network::DidDiscovery(value) => Some(Discovery::DidDiscovery(
					discovery::DidDiscovery::create(
						identity,
						&participant,
						value,
						DidDiscoveryMessage::Discover.to_string(),
					)
					.ok()?,
				)),
				Network::Rendezvous(value) => Some(Discovery::Rendezvous(value)),
				Network::Peer(value) => Some(Discovery::Peer(value)),
				_ => None,
			})
		});
	Ok(co_networks.chain(participant_networks).into_iter().collect())
}
