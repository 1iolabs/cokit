use crate::{library::find_membership::memberships, CoReducer};
use co_core_membership::Membership;
use co_primitives::{CoId, Did};

/// Find shared membership.
pub async fn shared_membership(
	parent: &CoReducer,
	co: &CoId,
	identity: Option<&Did>,
) -> Result<Option<Membership>, anyhow::Error> {
	// find first active membership
	Ok(memberships(&parent, &co).await?.find(move |membership| match identity {
		Some(value) => value == &membership.did,
		None => true,
	}))
}
