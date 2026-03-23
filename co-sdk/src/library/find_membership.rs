// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{
	state::{query_core, QueryExt},
	CoReducer, CO_CORE_NAME_MEMBERSHIP,
};
use co_core_membership::{Membership, MembershipState};
use co_primitives::{CoId, Did};
use co_storage::StorageError;

/// Find the first active [`Membership`] entry in `reducer` for `co`.
pub async fn find_membership(reducer: &CoReducer, co: impl AsRef<CoId>) -> Result<Option<Membership>, StorageError> {
	find_membership_by(reducer, co, None, Some(MembershipState::Active)).await
}

/// Find the [`Membership`] entry in `reducer` for `co` for `did` or any and `state` or any.
pub async fn find_membership_by(
	reducer: &CoReducer,
	co: impl AsRef<CoId>,
	did: Option<&Did>,
	state: Option<MembershipState>,
) -> Result<Option<Membership>, StorageError> {
	let (storage, memberships) = query_core(CO_CORE_NAME_MEMBERSHIP)
		.with_default()
		.execute_reducer(reducer)
		.await
		.map_err(Into::<StorageError>::into)?;
	let membership = memberships.memberships.get(&storage, co.as_ref()).await?;
	Ok(membership.filter(|membership| {
		membership
			.did
			.iter()
			.filter(move |(membership_did, _)| match did {
				Some(did) => *membership_did == did,
				None => true,
			})
			.any(move |(_, membership_state)| match &state {
				Some(state) => membership_state == state,
				None => true,
			})
	}))
}
