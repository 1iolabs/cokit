use super::{
	heads::{HeadsRequest, HeadsRequestNetworkTask},
	CoNetworkTaskSpawner,
};
use crate::{
	library::{co_peer_provider::CoPeerProvider, to_plain::to_plain},
	state, CoCoreResolver, CoReducer, CoStorage, Reducer, ReducerChangedHandler,
};
use anyhow::anyhow;
use async_trait::async_trait;
use co_network::PeerProvider;
use co_primitives::{CoId, NetworkCoHeads};
use co_storage::BlockStorageContentMapping;
use libipld::Cid;
use libp2p::PeerId;
use std::collections::BTreeSet;

/// Subscription for a single CO (`CoReducer`).
pub struct Subscription {
	spawner: CoNetworkTaskSpawner,
	co: CoId,
	network: NetworkCoHeads,
}
impl Subscription {
	pub(crate) async fn subscribe(
		spawner: CoNetworkTaskSpawner,
		co: CoReducer,
		network: NetworkCoHeads,
	) -> Result<Self, anyhow::Error> {
		spawner.spawn(HeadsRequestNetworkTask::new(HeadsRequest::Subscribe {
			network: network.clone(),
			co: co.id().clone(),
		}))?;
		Ok(Self { spawner, co: co.id().clone(), network })
	}

	pub fn unsubscribe(self) {
		self.spawner
			.spawn(HeadsRequestNetworkTask::new(HeadsRequest::Unsubscribe { network: self.network, co: self.co }))
			.ok();
	}
}

#[derive(Clone)]
pub struct Publish<M> {
	spawner: CoNetworkTaskSpawner,
	co: CoId,
	mapping: Option<M>,
	/// Force the mapping to be applied by returning an error when no mapping is found.
	force_mapping: bool,
}
impl<M> Publish<M> {
	pub fn new(spawner: CoNetworkTaskSpawner, co: CoId, mapping: Option<M>, force_mapping: bool) -> Self {
		Self { co, spawner, mapping, force_mapping }
	}

	// pub async fn request(&self, reducer: &CoReducer) -> Result<(), anyhow::Error>
	// where
	// 	M: BlockStorageContentMapping + Send + Sync + 'static,
	// {
	// 	let peers = CoPeerProvider::from_co_reducer(&reducer).await.peers().await?;
	// 	let mut heads = reducer.heads().await;

	// 	// map plain heads to encrypted heads
	// 	if self.mapping.is_some() {
	// 		heads = to_plain(&self.mapping, self.force_mapping, heads)
	// 			.await
	// 			.map_err(|err| anyhow!("Failed to map head: {}", err))?;
	// 	}

	// 	// request
	// 	self.request_peers(heads, peers)
	// }

	// pub fn request_peers(&self, heads: BTreeSet<Cid>, peers: BTreeSet<PeerId>) -> Result<(), anyhow::Error> {
	// 	self.spawner
	// 		.spawn(HeadsRequestNetworkTask::new(HeadsRequest::Heads { co: self.co.clone(), heads, peers }))?;
	// 	Ok(())
	// }
}
#[async_trait]
impl<M> ReducerChangedHandler<CoStorage, CoCoreResolver> for Publish<M>
where
	M: BlockStorageContentMapping + Send + Sync + 'static,
{
	// TODO: skip publish when have only one peer?
	async fn on_state_changed(&mut self, reducer: &Reducer<CoStorage, CoCoreResolver>) -> Result<(), anyhow::Error> {
		let mut heads = reducer.heads().clone();

		// map plain heads to encrypted heads
		if self.mapping.is_some() {
			heads = to_plain(&self.mapping, self.force_mapping, heads)
				.await
				.map_err(|err| anyhow!("Failed to map head: {}", err))?;
		}

		// networks
		let networks = state::networks(reducer.log().storage(), reducer.state().into()).await?;

		// publish
		for network in networks {
			match network {
				// co_api::Network::DidDiscovery(_) => todo!(),
				co_api::Network::CoHeads(network) => {
					self.spawner.spawn(HeadsRequestNetworkTask::new(HeadsRequest::PublishHeads {
						network,
						co: self.co.clone(),
						heads: heads.clone(),
					}))?;
				},
				// co_api::Network::Rendezvous(_) => todo!(),
				// co_api::Network::Peer(_) => todo!(),
				_ => {
					// ignore
				},
			}
		}

		// result
		Ok(())
	}
}
