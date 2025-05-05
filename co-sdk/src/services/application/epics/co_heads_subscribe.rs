use super::co_heads_publish::network_co_heads;
use crate::{
	network::{CoHeadsNetworkTask, CoHeadsRequest},
	types::co_reducer_context::CoReducerFeature,
	Action, CoContext, CoNetworkTaskSpawner, CoReducerFactory,
};
use co_actor::Epic;
use co_network::NetworkTaskSpawner;
use co_primitives::{CoId, NetworkCoHeads};
use futures::Stream;
use std::{
	collections::{BTreeSet, HashMap},
	sync::{Arc, Mutex},
};

#[derive(Debug, Clone, Default)]
pub struct CoHeadsSubscribeEpic {
	subscriptions: Arc<Mutex<HashMap<NetworkCoHeads, CoHeadsSubscription>>>,
}
impl Epic<Action, (), CoContext> for CoHeadsSubscribeEpic {
	fn epic(
		&mut self,
		action: &Action,
		_state: &(),
		context: &CoContext,
	) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
		let (co, mode) = match action {
			Action::CoOpen { co, network: true } => (co.clone(), true),
			Action::CoClose { co } => (co.clone(), false),
			_ => {
				return None;
			},
		};
		Some(Action::future_ignore_elements(subscribe(context.clone(), self.clone(), co, mode)))
	}
}

async fn subscribe(context: CoContext, epic: CoHeadsSubscribeEpic, co: CoId, mode: bool) -> Result<(), anyhow::Error> {
	// network
	let Some((spawner, _connections)) = context.network().await else {
		return Ok(());
	};

	// get current networks
	//  when mode is set to false we want to unsubscribe all from this co
	let networks: BTreeSet<NetworkCoHeads> = if mode {
		// get co
		let co_reducer = context.try_co_reducer(&co).await?;
		if !co_reducer.context.has_feature(&CoReducerFeature::Network) {
			return Ok(());
		}
		let co_reducer_state = co_reducer.reducer_state().await;
		let storage = co_reducer.storage();

		// get networks
		network_co_heads(&storage, co.clone(), co_reducer_state.co()).await?.collect()
	} else {
		Default::default()
	};

	// subscribe
	for network in networks.iter() {
		let mut subscriptions = epic.subscriptions.lock().unwrap();
		if !subscriptions.contains_key(&network) {
			subscriptions.insert(network.clone(), CoHeadsSubscription::subscribe(spawner.clone(), network.clone())?);
		}
	}

	// unsubscribe
	{
		let mut subscriptions = epic.subscriptions.lock().unwrap();
		let remove: Vec<NetworkCoHeads> = subscriptions
			.iter()
			.filter(|(key, _)| if &key.id == &co { !networks.contains(key) } else { false })
			.map(|(key, _)| key.to_owned())
			.collect();
		for key in remove.into_iter() {
			if let Some(subscription) = subscriptions.remove(&key) {
				subscription.unsubscribe();
			}
		}
	}

	// result
	Ok(())
}

/// Subscription for a single CO (`CoReducer`).
#[derive(Debug)]
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
