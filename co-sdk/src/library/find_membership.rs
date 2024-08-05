use crate::{CoReducer, CoReducerError, CO_CORE_NAME_MEMBERSHIP};
use co_core_membership::{Membership, MembershipState, Memberships};
use co_primitives::CoId;

/// Find the first active [`Membership`] entry in `reducer` for `co`.
pub async fn find_membership(reducer: &CoReducer, co: impl AsRef<CoId>) -> Result<Option<Membership>, CoReducerError> {
	Ok(memberships_iter(reducer, co).await?.next())
}

/// Find the active [`Membership`] entries in `reducer` for `co`.
pub async fn find_memberships(reducer: &CoReducer, co: impl AsRef<CoId>) -> Result<Vec<Membership>, CoReducerError> {
	Ok(memberships_iter(reducer, co).await?.collect())
}

/// Find the active [`Membership`] entries in `reducer` for `co`.
async fn memberships_iter<'a>(
	reducer: &CoReducer,
	co: impl AsRef<CoId> + 'a,
) -> Result<impl Iterator<Item = Membership> + 'a, CoReducerError> {
	let memberships: Memberships = match reducer.state(CO_CORE_NAME_MEMBERSHIP).await {
		Ok(memberships) => memberships,
		Err(CoReducerError::CoreNotFound(_)) => Memberships::default(),
		Err(e) => Err(e)?,
	};
	Ok(memberships
		.memberships
		.into_iter()
		.filter(move |membership| &membership.id == co.as_ref())
		.filter(|membership| membership.membership_state == MembershipState::Active)
		.into_iter())
}
