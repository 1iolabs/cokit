use crate::{CoReducer, CoReducerError, Cores, CO_CORE_MEMBERSHIP};
use co_core_membership::{Membership, Memberships};
use co_primitives::CoId;

/// Find the Membership entry in `reducer` for `co`.
pub async fn find_membership(reducer: &CoReducer, co: impl AsRef<CoId>) -> Result<Option<Membership>, CoReducerError> {
	let memberships: Memberships = match reducer.state(Cores::to_core_name(CO_CORE_MEMBERSHIP)).await {
		Ok(memberships) => memberships,
		Err(CoReducerError::CoreNotFound(_)) => Memberships::default(),
		Err(e) => Err(e)?,
	};
	for membership in memberships.memberships {
		if &membership.id == co.as_ref() {
			return Ok(Some(membership))
		}
	}
	Ok(None)
}

/// Find the Membership entry in `reducer` for `co`.
pub async fn find_memberships(reducer: &CoReducer, co: impl AsRef<CoId>) -> Result<Vec<Membership>, CoReducerError> {
	let memberships: Memberships = match reducer.state(Cores::to_core_name(CO_CORE_MEMBERSHIP)).await {
		Ok(memberships) => memberships,
		Err(CoReducerError::CoreNotFound(_)) => Memberships::default(),
		Err(e) => Err(e)?,
	};
	Ok(memberships
		.memberships
		.into_iter()
		.filter(|membership| &membership.id == co.as_ref())
		.collect())
}
