// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{library::find_membership::find_membership_by, CoReducer};
use co_core_membership::{Membership, MembershipState};
use co_primitives::{CoId, CoTryStreamExt, Did};
use futures::{StreamExt, TryStreamExt};

/// Find shared membership.
///
/// Selects the membership that equals the (optional) identity and the one with the lowest state.
pub async fn shared_membership(
	parent: &CoReducer,
	co: &CoId,
	identity: Option<&Did>,
) -> Result<Option<Membership>, anyhow::Error> {
	Ok(find_membership_by(parent, &co, identity, None).await?)
}

/// Find active shared membership.
pub async fn shared_membership_active(
	parent: &CoReducer,
	co: &CoId,
	identity: Option<&Did>,
) -> Result<Option<Membership>, anyhow::Error> {
	Ok(find_membership_by(parent, &co, identity, Some(MembershipState::Active)).await?)
}

/// Find active shared membership.
/// If it is not active yet wait for it to become active.
pub async fn wait_shared_membership_active(
	parent: &CoReducer,
	co: &CoId,
	identity: Option<&Did>,
) -> Result<Option<Membership>, anyhow::Error> {
	if let Some(membership) = shared_membership(parent, co, identity).await? {
		match membership.membership_state() {
			Some(MembershipState::Active) => Ok(Some(membership)),
			Some(MembershipState::Pending | MembershipState::Join) => {
				let result = parent
					.reducer_state_stream()
					.map(Ok)
					.try_filter_map(move |_parent_reducer_state| {
						let parent = parent.clone();
						let co = co.clone();
						let identity = identity.cloned();
						async move { shared_membership_active(&parent, &co, identity.as_ref()).await }
					})
					.try_first()
					.await;
				result
			},
			_ => Ok(None),
		}
	} else {
		Ok(None)
	}
}
