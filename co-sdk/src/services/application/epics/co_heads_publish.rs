use crate::{
	network::{CoHeadsNetworkTask, CoHeadsRequest},
	state,
	types::co_reducer_context::CoReducerFeature,
	Action, CoContext, CoReducerFactory, CoStorage,
};
use co_actor::Actions;
use co_core_co::Co;
use co_network::NetworkTaskSpawner;
use co_primitives::{CoId, Network, NetworkCoHeads, OptionLink};
use futures::Stream;

/// Publish heads to a gossip sub topic when a network reducer has flushed.
pub fn co_heads_publish(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	let co = match action {
		Action::CoFlush { co, info } if info.network => co.clone(),
		_ => {
			return None;
		},
	};
	Some(Action::future_ignore_elements(publish(context.clone(), co)))
}

async fn publish(context: CoContext, co: CoId) -> Result<(), anyhow::Error> {
	// network
	let Some((spawner, _connections)) = context.network().await else {
		return Ok(());
	};

	// get co
	let co_reducer = context.try_co_reducer(&co).await?;
	let co_reducer_state = co_reducer.reducer_state().await;
	let storage = co_reducer.storage();

	// map plain heads to encrypted heads
	let external_co_reducer_state = if co_reducer.context.has_feature(&CoReducerFeature::Encryption) {
		Some(co_reducer_state.to_external_force(&storage).await?)
	} else {
		None
	};

	// publish
	for network in network_co_heads(&storage, co.clone(), co_reducer_state.co()).await? {
		// publish
		spawner.spawn(CoHeadsNetworkTask::new(CoHeadsRequest::PublishHeads {
			network: network.clone(),
			heads: external_co_reducer_state.as_ref().unwrap_or(&co_reducer_state).heads(),
		}))?;
	}
	Ok(())
}

pub async fn network_co_heads<'a>(
	storage: &CoStorage,
	co: CoId,
	state: OptionLink<Co>,
) -> Result<impl Iterator<Item = NetworkCoHeads>, anyhow::Error> {
	Ok(state::networks(storage, state)
		.await?
		.into_iter()
		.filter_map(move |network| match network {
			Network::CoHeads(network) if network.id == co => Some(network),
			_ => None,
		}))
}
