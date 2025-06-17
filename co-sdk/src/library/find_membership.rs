use crate::{
	state::{query_core, QueryExt},
	CoReducer, CO_CORE_NAME_MEMBERSHIP,
};
use co_core_membership::{Membership, MembershipState};
use co_primitives::CoId;
use co_storage::StorageError;

/// Find the first active [`Membership`] entry in `reducer` for `co`.
pub async fn find_membership(reducer: &CoReducer, co: impl AsRef<CoId>) -> Result<Option<Membership>, StorageError> {
	Ok(memberships(reducer, co).await?.next())
}

/// Find the active [`Membership`] entries in `reducer` for `co`.
pub async fn find_memberships(reducer: &CoReducer, co: impl AsRef<CoId>) -> Result<Vec<Membership>, StorageError> {
	Ok(memberships(reducer, co).await?.collect())
}

/// Find the active [`Membership`] entries in `reducer` for `co`.
pub async fn memberships<'a>(
	reducer: &CoReducer,
	co: impl AsRef<CoId> + 'a,
) -> Result<impl Iterator<Item = Membership> + 'a, StorageError> {
	let (_, memberships) = query_core(CO_CORE_NAME_MEMBERSHIP)
		.with_default()
		.execute_reducer(&reducer)
		.await
		.map_err(Into::<StorageError>::into)?;
	Ok(memberships
		.memberships
		.into_iter()
		.filter(move |membership| &membership.id == co.as_ref())
		.filter(|membership| membership.membership_state == MembershipState::Active)
		.into_iter())
}
