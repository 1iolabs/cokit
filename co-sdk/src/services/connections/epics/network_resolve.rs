use crate::{
	find_membership,
	library::{invite_networks::invite_networks, network_discovery::identities_networks},
	services::connections::{ConnectionAction, ConnectionState, NetworkResolveAction, NetworkResolvedAction},
	state, CoContext, CoReducer, CoStorage,
};
use anyhow::anyhow;
use co_actor::Epic;
use co_primitives::{CoId, CoInviteMetadata, KnownTags, Network};
use co_storage::BlockStorageExt;
use futures::{Stream, TryStreamExt};
use std::{collections::BTreeSet, time::Instant};

pub struct NetworkResolveEpic();
impl NetworkResolveEpic {
	pub fn new() -> Self {
		Self()
	}
}
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
					yield ConnectionAction::NetworkResolved(NetworkResolvedAction { id: id.clone(), result, time: Instant::now() })
				})
			},
			_ => None,
		}
	}
}

async fn network_resolve(context: CoContext, id: CoId) -> Result<BTreeSet<Network>, anyhow::Error> {
	// to prevent deadlocking we want to use the storage without networking
	let reducers = context.inner.reducers_control();
	let storage = reducers.storage(id.clone()).await?.storage().clone();

	// reducer
	let reducer = reducers.reducer(id.clone()).await?;

	// get CO networks
	// - or participant networks if CO networks are empty
	// - or invite metadata if the previous fail (because the block is not loaded yet)
	match networks_co(&context, &storage, &reducer).await {
		Ok(networks) => Ok(networks),
		Err(err) => {
			tracing::debug!(?err, co = ?id, "co-resolve-networks-failed (fallback to invite)");
			Ok(networks_invite(&context, &id).await?)
		},
	}
}

/// Get CO Network settings.
async fn networks_co(
	context: &CoContext,
	storage: &CoStorage,
	reducer: &CoReducer,
) -> Result<BTreeSet<Network>, anyhow::Error> {
	let co_state = reducer.co_state().await;
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

/// Get CO Networks setting from invite.
async fn networks_invite(context: &CoContext, id: &CoId) -> Result<BTreeSet<Network>, anyhow::Error> {
	// get membership
	let local_co = context.local_co_reducer().await?;
	let membership = find_membership(&local_co, id)
		.await?
		.ok_or(anyhow!("No membership found: {id}"))?;

	// get metadata
	let invite_cid = membership
		.tags
		.link(&KnownTags::CoInviteMetadata.to_string())
		.ok_or(anyhow!("No co-invite-metadata"))?;
	let invite: CoInviteMetadata = local_co.storage().get_deserialized(invite_cid).await?;

	// get networks
	Ok(invite_networks(&context, &invite).await?)
}
