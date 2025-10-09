use crate::{
	find_membership,
	library::{
		invite_networks::invite_networks, network_discovery::identities_networks, shared_membership::shared_membership,
	},
	services::{
		connections::{ConnectionAction, ConnectionState, NetworkResolveAction, NetworkResolvedAction},
		reducers::ReducerOptions,
	},
	state, CoContext, CoReducer, CoReducerFactoryResultExt, CoStorage,
};
use anyhow::anyhow;
use co_actor::{Actions, Epic};
use co_core_membership::MembershipState;
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
		_actions: &Actions<ConnectionAction, ConnectionState, CoContext>,
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
	// check membership to know if we can read networks from co
	// Note: find membership assumes that its local maybe change that later.
	let local_co = context.local_co_reducer().await?;
	let membership = shared_membership(&local_co, &id, None)
		.await?
		.ok_or(anyhow!("No membership found: {:?}", id))?;
	let skip_co = match membership.membership_state {
		MembershipState::Join => true,
		MembershipState::Invite => true,
		MembershipState::Inactive => true,
		_ => false,
	};

	// this may gets called while reducer is being initialized
	//  to prevent deadlocking:
	//  - we skip the lookup while the item is created and fallback or error early
	//  - we want to use the storage without networking
	//  also we want to ignore storage creation errors and fallback to other strategies (invite)
	if skip_co {
		let reducers = context.inner.reducers_control();
		if let Ok(Some(reducer_storage)) = reducers
			.storage(id.clone(), ReducerOptions::default().with_no_pending_create())
			.await
			.opt()
		{
			// storage
			let storage = reducer_storage.storage();

			// reducer
			//  this should work in any case when we already got the storage
			//  may only fails if need to fetch heads from network?
			let reducer = reducers.reducer(id.clone(), Default::default()).await?;

			// get CO networks
			// - or participant networks if CO networks are empty
			// - or invite metadata if the previous fail (because the block is not loaded yet)
			match networks_co(&context, storage, &reducer).await {
				Ok(networks) => {
					return Ok(networks);
				},
				Err(err) => {
					tracing::warn!(?err, co = ?id, "co-resolve-networks-failed (fallback to invite)");
				},
			}
		}
	}

	// fallback to invite network settings immediately if the reducer is requesting from network while initializing
	//  this should only happen on invite
	//  as for every other thing the co root is always available
	//  because we need it to actually write it
	match networks_invite(&context, &id).await {
		Ok(networks) => {
			if !networks.is_empty() {
				tracing::debug!(co = ?id, ?networks, "co-resolve-networks-invite");
				return Ok(networks);
			}
		},
		_ => {},
	}

	// fail
	Err(anyhow!("Resolve networks failed"))
}

/// Get CO Network settings.
async fn networks_co(
	context: &CoContext,
	storage: &CoStorage,
	reducer: &CoReducer,
) -> Result<BTreeSet<Network>, anyhow::Error> {
	let co_state = reducer.reducer_state().await.co();
	let networks = state::networks(&storage, co_state).await?;
	if networks.is_empty() {
		// get participant networks
		let identity_resolver = context.identity_resolver().await?;
		let participants = state::participants_active(&storage, co_state).await?;
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
