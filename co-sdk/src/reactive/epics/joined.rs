use crate::{
	reactive::context::{ActionObservable, StateObservable},
	state, Action, CoContext, CoReducerFactory, CO_CORE_NAME_MEMBERSHIP, CO_ID_LOCAL,
};
use co_core_membership::{MembershipState, MembershipsAction};
use co_primitives::{CoId, Did};
use futures::{Stream, StreamExt, TryStreamExt};
use std::future::ready;

/// Fetch co core state and set membership to active when joined or back to invite when failed.
/// TODO: validate consensus?
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
				// override peer provider to only use the known peer until we fetched the network settings
				if let Some(peer) = peer {
					context
						.inner
						.network_overrides()
						.set(id.clone(), [peer].into_iter().collect())
						.await;
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

/// Fetch participants and network settings when join CO.
pub fn joined_fetch(
	actions: ActionObservable,
	_states: StateObservable,
	context: CoContext,
) -> impl Stream<Item = Action> + Send + 'static {
	actions
		.filter_map(|action| async move {
			match action {
				Action::CoreAction { co, context: _, action, cid: _ }
					if co.as_str() == CO_ID_LOCAL && action.core == CO_CORE_NAME_MEMBERSHIP =>
				{
					let membership_action: MembershipsAction = action.get_payload().ok()?;
					match membership_action {
						MembershipsAction::ChangeMembershipState {
							id,
							did,
							membership_state: MembershipState::Active,
						} => Some((id, did)),
						_ => None,
					}
				},
				_ => None,
			}
		})
		.then(move |(id, did)| {
			let context = context.clone();
			async move {
				// fetch
				joined_initialize(&context, &id, did).await?;

				// remove override
				context.inner.network_overrides().remove(&id).await;

				// done
				Ok([])
			}
		})
		.flat_map(Action::map_error_stream::<anyhow::Error>)
}

/// Initialize the joined CO.
///
/// We fetch at least the co state with networks and participants so we can reconnect later.
#[tracing::instrument(err, skip(context))]
async fn joined_initialize(context: &CoContext, id: &CoId, did: Did) -> anyhow::Result<()> {
	let co_reducer = context.co_reducer(&id).await?.ok_or(anyhow::anyhow!("Co not found: {}", id))?;

	// fetch co
	let co = co_reducer.co().await?;

	// fetch network settings and participants
	state::stream(co_reducer.storage(), &co.network).try_collect::<Vec<_>>().await?;
	// TODO: participants DAG (https://gitlab.1io.com/1io/co-sdk/-/issues/39)
	// state::stream(co_reducer.storage(), &co.participants).try_collect::<Vec<_>>().await?;

	// fetch
	Ok(())
}
