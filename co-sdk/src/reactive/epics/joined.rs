use crate::{
	reactive::context::{ActionObservable, StateObservable},
	Action, CoContext, CO_CORE_NAME_MEMBERSHIP, CO_ID_LOCAL,
};
use anyhow::anyhow;
use co_core_membership::{Membership, Memberships, MembershipsAction};
use co_identity::PrivateIdentityResolver;
use co_network::{bitswap::NetworkBlockStorage, StaticPeerProvider};
use co_primitives::{CoId, Did};
use co_storage::BlockStorage;
use futures::{Stream, StreamExt};
use libp2p::PeerId;
use std::{future::ready, time::Duration};

/// Fetch co core state and set membership to active when joined or back to invite when failed.
pub fn joined(
	actions: ActionObservable,
	_states: StateObservable,
	context: CoContext,
) -> impl Stream<Item = Action> + Send + 'static {
	actions
		.clone()
		.filter_map(|action| {
			ready(match action {
				Action::Joined { co, participant, success, peer } => Some((co, participant, success, peer)),
				_ => None,
			})
		})
		.then(move |(id, did, success, peer)| {
			let context = context.clone();
			async move {
				// fetch
				if let Some(peer) = peer {
					fetch_state_and_heads(context, id.clone(), did.clone(), peer).await?;
				}

				// active
				let payload = MembershipsAction::ChangeMembershipState {
					id: id.clone(),
					did: did.clone(),
					membership_state: if success {
						co_core_membership::MembershipState::Active
					} else {
						co_core_membership::MembershipState::Invite
					},
				};
				Ok(Action::push(CO_ID_LOCAL, did, CO_CORE_NAME_MEMBERSHIP, payload))
			}
		})
		.map(Action::map_error::<anyhow::Error>)
}

async fn fetch_state_and_heads(context: CoContext, id: CoId, did: Did, peer: PeerId) -> anyhow::Result<()> {
	let local_co = context.local_co_reducer().await?;
	let membership =
		find_membership(&context, &id, &did)
			.await?
			.ok_or(anyhow::anyhow!("Membership not found: {} ({})", id, did))?;
	// let co_reducer = context.co_reducer(&id).await?.ok_or(anyhow::anyhow!("Co not found: {}", id))?;
	let identity = context.private_identity_resolver().await?.resolve_private(&did).await?;
	let co_reducer = context
		.inner
		.create_co_instance_membership(local_co, membership, identity, None, false, false)
		.await?;
	let network = context.network().await.ok_or(anyhow!("Expected network"))?;
	let storage = NetworkBlockStorage::new(
		co_reducer.storage(),
		network,
		StaticPeerProvider::new([peer].into_iter().collect()),
		Duration::from_secs(30),
	);
	let (state, heads) = co_reducer.reducer_state().await;
	for cid in state.into_iter().chain(heads) {
		storage.get(&cid).await?;
	}
	Ok(())
}

async fn find_membership(context: &CoContext, id: &CoId, did: &Did) -> anyhow::Result<Option<Membership>> {
	let local = context.local_co_reducer().await?;
	let memberships: Memberships = local.state(CO_CORE_NAME_MEMBERSHIP).await?;
	Ok(memberships
		.memberships
		.into_iter()
		.find(|membership| &membership.id == id && &membership.did == did))
}
