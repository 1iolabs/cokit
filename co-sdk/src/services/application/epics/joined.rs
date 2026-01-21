use crate::{state, Action, CoContext, CoReducerFactory, CO_CORE_NAME_MEMBERSHIP, CO_ID_LOCAL};
use co_actor::Actions;
use co_core_membership::{MembershipState, MembershipsAction};
use co_primitives::{CoId, Did};
use futures::{stream, Stream, StreamExt, TryStreamExt};
use std::future::ready;

/// Fetch co core state and set membership to active when joined or back to invite when failed.
/// TODO: validate consensus?
/// In: [`Action::Joined`]
/// Out: [`Action::CoreActionPush`]
pub fn joined(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::Joined { co, participant, success, peer: _ } => Some(stream::once(ready({
			// active
			let payload = MembershipsAction::ChangeMembershipState {
				id: co.clone(),
				did: participant.clone(),
				membership_state: if *success {
					co_core_membership::MembershipState::Active
				} else {
					co_core_membership::MembershipState::Invite
				},
			};
			Ok(Action::push(CO_ID_LOCAL, participant, CO_CORE_NAME_MEMBERSHIP, payload, context.date()))
		}))),
		_ => None,
	}
}

/// Fetch participants and network settings when join CO.
/// In: [`Action::CoreAction`]
pub fn joined_fetch(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::CoreAction { co, storage: _, context: _, action, cid: _, head: _ }
			if co.as_str() == CO_ID_LOCAL && CO_CORE_NAME_MEMBERSHIP == action.core =>
		{
			let membership_action: MembershipsAction = action.get_payload().ok()?;
			match membership_action {
				MembershipsAction::ChangeMembershipState { id, did, membership_state: MembershipState::Active } => {
					Some(
						stream::once({
							let context = context.clone();
							async move {
								// fetch
								joined_initialize(&context, &id, did).await?;

								// done
								Ok(())
							}
						})
						.filter_map(Action::filter_map_error)
						.map(Ok),
					)
				},
				_ => None,
			}
		},
		_ => None,
	}
}

/// Initialize the joined CO.
///
/// We fetch at least the co state with networks and participants so we can reconnect later.
#[tracing::instrument(level = tracing::Level::TRACE, err, skip(context))]
async fn joined_initialize(context: &CoContext, id: &CoId, did: Did) -> anyhow::Result<()> {
	let co_reducer = context.co_reducer(id).await?.ok_or(anyhow::anyhow!("Co not found: {}", id))?;

	// fetch co
	let (storage, co) = co_reducer.co().await?;

	// fetch network settings and participants
	state::stream(storage, &co.network).try_collect::<Vec<_>>().await?;
	// TODO: participants DAG (https://gitlab.1io.com/1io/co-sdk/-/issues/39)
	// state::stream(co_reducer.storage(), &co.participants).try_collect::<Vec<_>>().await?;

	// fetch
	Ok(())
}
