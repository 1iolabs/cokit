use super::{
	heads::{HeadsRequest, HeadsRequestNetworkTask},
	CoNetworkTaskSpawner,
};
use crate::{
	library::co_peer_provider::CoPeerProvider, CoCoreResolver, CoReducer, CoStorage, Reducer, ReducerChangedHandler,
};
use anyhow::anyhow;
use async_trait::async_trait;
use co_network::PeerProvider;
use co_primitives::CoId;
use co_storage::BlockStorageContentMapping;
use futures::{StreamExt, TryStreamExt};
use libipld::Cid;
use libp2p::PeerId;
use std::collections::BTreeSet;

/// Subscription for a single CO (`CoReducer`).
pub struct Subscription {
	spawner: CoNetworkTaskSpawner,
	co: CoId,
}
impl Subscription {
	pub(crate) async fn subscribe(spawner: CoNetworkTaskSpawner, co: CoReducer) -> Result<Self, anyhow::Error> {
		spawner.spawn(HeadsRequestNetworkTask::new(HeadsRequest::Subscribe { co: co.id().clone() }))?;
		Ok(Self { spawner, co: co.id().clone() })
	}

	pub fn unsubscribe(self) {
		self.spawner
			.spawn(HeadsRequestNetworkTask::new(HeadsRequest::Unsubscribe { co: self.co }))
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

	async fn to_plain(&self, head: Cid) -> Result<Cid, anyhow::Error>
	where
		M: BlockStorageContentMapping + Send + Sync + 'static,
	{
		match &self.mapping {
			Some(mapping) => match mapping.to_plain(&head).await {
				Some(cid) => Ok(cid),
				None if self.force_mapping => Err(anyhow!("Failed to map: {:?}", head)),
				None => Ok(head),
			},
			None => Ok(head),
		}
	}

	pub async fn request(&self, reducer: &CoReducer) -> Result<(), anyhow::Error>
	where
		M: BlockStorageContentMapping + Send + Sync + 'static,
	{
		let peers = CoPeerProvider::from_co_reducer(&reducer).await.peers().await?;
		let mut heads = reducer.heads().await;

		// map plain heads to encrypted heads
		if self.mapping.is_some() {
			heads = futures::stream::iter(heads.into_iter())
				.then(|head| self.to_plain(head))
				.try_collect()
				.await?;
		}

		// request
		self.request_peers(heads, peers)
	}

	pub fn request_peers(&self, heads: BTreeSet<Cid>, peers: BTreeSet<PeerId>) -> Result<(), anyhow::Error> {
		self.spawner
			.spawn(HeadsRequestNetworkTask::new(HeadsRequest::Heads { co: self.co.clone(), heads, peers }))?;
		Ok(())
	}
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
			heads = futures::stream::iter(heads.into_iter())
				.then(|head| self.to_plain(head))
				.try_collect()
				.await?;
		}

		// publish
		self.spawner
			.spawn(HeadsRequestNetworkTask::new(HeadsRequest::PublishHeads { co: self.co.clone(), heads }))?;

		// result
		Ok(())
	}
}
