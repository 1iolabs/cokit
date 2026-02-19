// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{
	state::{query_core, QueryExt},
	CoReducer, CO_CORE_NAME_MEMBERSHIP,
};
use co_core_membership::{Membership, MembershipState};
use co_primitives::CoId;
use co_storage::StorageError;

/// Find the first active [`Membership`] entry in `reducer` for `co`.
pub async fn find_membership(reducer: &CoReducer, co: impl AsRef<CoId>) -> Result<Option<Membership>, StorageError> {
	Ok(memberships(reducer, co, Some(MembershipState::Active)).await?.next())
}

/// Find the active [`Membership`] entries in `reducer` for `co`.
pub async fn find_memberships(reducer: &CoReducer, co: impl AsRef<CoId>) -> Result<Vec<Membership>, StorageError> {
	Ok(memberships(reducer, co, Some(MembershipState::Active)).await?.collect())
}

/// Find the [`Membership`] entries in `reducer` for `co`.
/// Optionally filtered by `state`.
pub async fn memberships<'a>(
	reducer: &CoReducer,
	co: impl AsRef<CoId> + 'a,
	state: Option<MembershipState>,
) -> Result<impl Iterator<Item = Membership> + 'a, StorageError> {
	let (_, memberships) = query_core(CO_CORE_NAME_MEMBERSHIP)
		.with_default()
		.execute_reducer(reducer)
		.await
		.map_err(Into::<StorageError>::into)?;
	Ok(memberships
		.memberships
		.into_iter()
		.filter(move |membership| &membership.id == co.as_ref())
		.filter(move |membership| if let Some(state) = state { membership.membership_state == state } else { true }))
}
