use crate::{
	reactive::context::{ActionObservable, StateObservable},
	Action, CoContext, CO_CORE_NAME_MEMBERSHIP, CO_ID_LOCAL,
};
use co_core_membership::MembershipsAction;
use futures::{Stream, StreamExt};
use std::future::ready;

/// Set membership to active when joined or back to invite when failed
pub fn joined(
	actions: ActionObservable,
	_states: StateObservable,
	_context: CoContext,
) -> impl Stream<Item = Action> + Send + 'static {
	actions
		.clone()
		.filter_map(|action| {
			ready(match action {
				Action::Joined { co, participant, success } => Some((co, participant, success)),
				_ => None,
			})
		})
		.then(move |(id, did, success)| {
			ready({
				let payload = MembershipsAction::ChangeMembershipState {
					id: id.clone(),
					did: did.clone(),
					membership_state: if success {
						co_core_membership::MembershipState::Active
					} else {
						co_core_membership::MembershipState::Invite
					},
				};
				Action::push(CO_ID_LOCAL, did, CO_CORE_NAME_MEMBERSHIP, payload)
			})
		})
}
