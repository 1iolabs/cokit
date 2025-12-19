use crate::{
	find_membership,
	library::{invite_networks::invite_networks, shared_membership::shared_membership},
	services::reducers::ReducerOptions,
	state, CoContext, CoReducer, CoReducerFactoryResultExt, CoStorage,
};
use anyhow::anyhow;
use async_trait::async_trait;
use co_core_membership::MembershipState;
use co_network::{connections::NetworkResolver, identities_networks};
use co_primitives::{CoId, CoInviteMetadata, KnownTags, Network};
use co_storage::BlockStorageExt;
use futures::TryStreamExt;
use std::collections::BTreeSet;

#[derive(Debug)]
pub struct CoNetworkResolver {
	context: CoContext,
}
impl CoNetworkResolver {
	pub fn new(context: CoContext) -> Self {
		Self { context }
	}
}
#[async_trait]
impl NetworkResolver for CoNetworkResolver {
	async fn networks(&self, id: CoId) -> Result<BTreeSet<Network>, anyhow::Error> {
		// check membership to know if we can read networks from co
		// Note: find membership assumes that its local maybe change that later.
		let local_co = self.context.local_co_reducer().await?;
		let membership = shared_membership(&local_co, &id, None)
			.await?
			.ok_or(anyhow!("No membership found: {:?}", id))?;
		let use_co = match membership.membership_state {
			MembershipState::Join | MembershipState::Invite | MembershipState::Inactive => false,
			MembershipState::Active | _ => true,
		};

		// this may gets called while reducer is being initialized
		//  to prevent deadlocking:
		//  - we skip the lookup while the item is created and fallback or error early
		//  - we want to use the storage without networking
		//  also we want to ignore storage creation errors and fallback to other strategies (invite)
		if use_co {
			let reducers = self.context.inner.reducers_control();
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
				match networks_co(&self.context, storage, &reducer).await {
					Ok(networks) => {
						return Ok(networks);
					},
					Err(err) => {
						tracing::warn!(?err, co = ?id, "co-resolve-networks-failed (fallback to invite)");
					},
				}
			}
		}

		// fallback to invite network settings immediately if the reducer is requesting from network while
		// initializing  this should only happen on invite
		//  as for every other thing the co root is always available
		//  because we need it to actually write it
		if let Ok(networks) = networks_invite(&self.context, &id).await {
			if !networks.is_empty() {
				tracing::debug!(co = ?id, ?networks, "co-resolve-networks-invite");
				return Ok(networks);
			}
		}

		// fail
		Err(anyhow!("Resolve networks failed"))
	}
}

/// Get CO Network settings.
async fn networks_co(
	context: &CoContext,
	storage: &CoStorage,
	reducer: &CoReducer,
) -> Result<BTreeSet<Network>, anyhow::Error> {
	let co_state = reducer.reducer_state().await.co();
	let networks = state::networks(storage, co_state).await?;
	if networks.is_empty() {
		// get participant networks
		let identity_resolver = context.identity_resolver().await?;
		let participants = state::participants_active(storage, co_state).await?;
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
	invite_networks(context, &invite).await
}
