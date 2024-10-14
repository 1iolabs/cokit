use crate::{
	actor::Epic,
	library::network_discovery::identities_networks,
	plugins::connections::{ConnectionAction, ConnectionState, NetworkResolveAction, NetworkResolvedAction},
	state, CoContext,
};
use co_primitives::{CoId, Network};
use futures::{Stream, TryStreamExt};
use std::collections::BTreeSet;

pub struct NetworkResolveEpic();
impl Epic<ConnectionAction, ConnectionState, CoContext> for NetworkResolveEpic {
	fn epic(
		&mut self,
		message: &ConnectionAction,
		_state: &ConnectionState,
		context: &CoContext,
	) -> Option<impl Stream<Item = Result<ConnectionAction, anyhow::Error>> + 'static> {
		match message {
			ConnectionAction::NetworkResolve(NetworkResolveAction { id }) => {
				let context = context.clone();
				let id = id.clone();
				Some(async_stream::try_stream! {
					let result = network_resolve(context, id.clone()).await.map_err(|err| err.to_string());
					yield ConnectionAction::NetworkResolved(NetworkResolvedAction { id: id.clone(), result })
				})
			},
			_ => None,
		}
	}
}

async fn network_resolve(context: CoContext, id: CoId) -> Result<BTreeSet<Network>, anyhow::Error> {
	// to prevent deadlocking we want to use the storage without networking
	let mut reducers = context.inner.reducers_control();
	let storage = reducers.storage(id.clone()).await?.storage();

	// reducer
	let reducer = reducers.reducer(id.clone()).await?;
	let co_state = reducer.co_state().await;

	// get CO networks (or participant networks if CO networks are empty)
	let networks = state::networks(&storage, co_state).await?;
	if networks.is_empty() {
		// get participant networks
		let identity_resolver = context.identity_resolver().await?;
		let participants = state::participants(&storage, co_state).await?;
		Ok(identities_networks(Some(&identity_resolver), participants.into_iter().map(|item| item.did))
			.try_collect()
			.await?)
	} else {
		Ok(networks.into_iter().collect())
	}
}
