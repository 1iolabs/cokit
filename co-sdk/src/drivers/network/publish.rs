use super::{
	tasks::co_heads::{CoHeadsNetworkTask, CoHeadsRequest},
	CoNetworkTaskSpawner,
};
use crate::{
	library::to_plain::to_plain, reducer::core_resolver::dynamic::DynamicCoreResolver, state, CoStorage, Reducer,
	ReducerChangeContext, ReducerChangedHandler,
};
use anyhow::anyhow;
use async_trait::async_trait;
use co_network::NetworkTaskSpawner;
use co_primitives::{CoId, Network, NetworkCoHeads};
use co_storage::BlockStorageContentMapping;
use std::collections::{BTreeMap, BTreeSet};

/// Subscribe, Unsubscribe and Publish to CoHeads protocol when a reducer changes.
/// Subscriptions will be unsubscribed when dropped (also when the reduer is dropped).
pub struct CoHeadsPublish<M> {
	spawner: CoNetworkTaskSpawner,
	co: CoId,
	mapping: Option<M>,
	/// Force the mapping to be applied by returning an error when no mapping is found.
	force_mapping: bool,
	subscriptions: BTreeMap<NetworkCoHeads, CoHeadsSubscription>,
}
impl<M> CoHeadsPublish<M> {
	pub fn new(spawner: CoNetworkTaskSpawner, co: CoId, mapping: Option<M>, force_mapping: bool) -> Self {
		Self { co, spawner, mapping, force_mapping, subscriptions: Default::default() }
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
impl<M> ReducerChangedHandler<CoStorage, DynamicCoreResolver<CoStorage>> for CoHeadsPublish<M>
where
	M: BlockStorageContentMapping + Send + Sync + 'static,
{
	// TODO: skip publish when have only one peer?
	async fn on_state_changed(
		&mut self,
		reducer: &Reducer<CoStorage, DynamicCoreResolver<CoStorage>>,
		_context: ReducerChangeContext,
	) -> Result<(), anyhow::Error> {
		let mut heads = reducer.heads().clone();

		// map plain heads to encrypted heads
		if self.mapping.is_some() {
			heads = to_plain(&self.mapping, self.force_mapping, heads)
				.await
				.map_err(|err| anyhow!("Failed to map head: {}", err))?;
		}

		// networks
		let networks: BTreeSet<_> = state::networks(reducer.log().storage(), reducer.state().into())
			.await?
			.into_iter()
			.filter_map(|network| match network {
				Network::CoHeads(network) if network.id == self.co => Some(network),
				_ => None,
			})
			.collect();
		for network in networks.iter() {
			// subscribe
			if !self.subscriptions.contains_key(network) {
				self.subscriptions
					.insert(network.clone(), CoHeadsSubscription::subscribe(self.spawner.clone(), network.clone())?);
			}

			// publish
			self.spawner.spawn(CoHeadsNetworkTask::new(CoHeadsRequest::PublishHeads {
				network: network.clone(),
				heads: heads.clone(),
			}))?;
		}

		// unsubscribe
		if self.subscriptions.len() != networks.len() {
			let remove: Vec<_> = self
				.subscriptions
				.iter()
				.filter(|(key, _)| !networks.contains(key))
				.map(|(key, _)| key.to_owned())
				.collect();
			for key in remove.into_iter() {
				if let Some(subscription) = self.subscriptions.remove(&key) {
					subscription.unsubscribe();
				}
			}
		}

		// result
		Ok(())
	}
}

/// Subscription for a single CO (`CoReducer`).
struct CoHeadsSubscription {
	spawner: CoNetworkTaskSpawner,
	unsubscribe: Option<CoHeadsNetworkTask>,
}
impl CoHeadsSubscription {
	pub fn subscribe(spawner: CoNetworkTaskSpawner, network: NetworkCoHeads) -> Result<Self, anyhow::Error> {
		spawner.spawn(CoHeadsNetworkTask::new(CoHeadsRequest::Subscribe { network: network.clone() }))?;
		Ok(Self { spawner, unsubscribe: Some(CoHeadsNetworkTask::new(CoHeadsRequest::Unsubscribe { network })) })
	}

	pub fn unsubscribe(self) {
		drop(self);
	}
}
impl Drop for CoHeadsSubscription {
	fn drop(&mut self) {
		if let Some(unsubscribe) = self.unsubscribe.take() {
			self.spawner.spawn(unsubscribe).ok();
		}
	}
}
