use crate::{library::find_membership::memberships, CoReducer};
use co_core_membership::{Membership, MembershipState};
use co_primitives::{CoId, Did};
use std::cmp::Ordering;

/// Find shared membership.
///
/// Selects the membership that equals the (optional) identity and the one with the lowest state.
pub async fn shared_membership(
	parent: &CoReducer,
	co: &CoId,
	identity: Option<&Did>,
) -> Result<Option<Membership>, anyhow::Error> {
	let mut result = None;
	for membership in memberships(parent, &co, None).await? {
		result = Some(match result {
			None => membership,
			Some(prev) => match sort_membership(identity, &membership, &prev) {
				Ordering::Greater => membership,
				_ => prev,
			},
		});
	}
	Ok(result)
}

/// Find active shared membership.
pub async fn shared_membership_active(
	parent: &CoReducer,
	co: &CoId,
	identity: Option<&Did>,
) -> Result<Option<Membership>, anyhow::Error> {
	// find first active membership
	Ok(memberships(parent, &co, Some(MembershipState::Active))
		.await?
		.find(move |membership| match identity {
			Some(value) => value == &membership.did,
			None => true,
		}))
}

fn sort_membership(identity: Option<&Did>, a: &Membership, b: &Membership) -> Ordering {
	let a_is_identity = if let Some(identity) = identity { identity == &a.did } else { false };
	let b_is_identity = if let Some(identity) = identity { identity == &b.did } else { false };
	let is_identity = a_is_identity.cmp(&b_is_identity);
	if is_identity == Ordering::Equal {
		a.membership_state.cmp(&b.membership_state).reverse()
	} else {
		is_identity
	}
}

#[cfg(test)]
mod tests {
	use crate::library::shared_membership::sort_membership;
	use co_core_membership::{Membership, MembershipState};
	use co_primitives::CoId;
	use std::cmp::Ordering;

	#[test]
	fn test_sort_membership() {
		let identity1 = "did:example:1".to_string();
		let identity2 = "did:example:2".to_string();
		let identity3 = "did:example:3".to_string();
		let membership1 = Membership {
			did: identity1.clone(),
			membership_state: MembershipState::Active,
			id: CoId::new(""),
			state: Default::default(),
			key: None,
			tags: Default::default(),
		};
		let membership2 = Membership {
			did: identity2.clone(),
			membership_state: MembershipState::Inactive,
			id: CoId::new(""),
			state: Default::default(),
			key: None,
			tags: Default::default(),
		};
		let membership3 = Membership {
			did: identity1.clone(),
			membership_state: MembershipState::Invite,
			id: CoId::new(""),
			state: Default::default(),
			key: None,
			tags: Default::default(),
		};
		let membership4 = Membership {
			did: identity1.clone(),
			membership_state: MembershipState::Join,
			id: CoId::new(""),
			state: Default::default(),
			key: None,
			tags: Default::default(),
		};

		// identity is None
		assert_eq!(
			sort_membership(None, &membership1, &membership2),
			Ordering::Greater // active has lower state than inactive
		);
		assert_eq!(
			sort_membership(None, &membership2, &membership3),
			Ordering::Less // invite has higher state than inactive
		);

		// identity is Some
		assert_eq!(
			sort_membership(Some(&identity1), &membership1, &membership3),
			Ordering::Greater // active (with matching identity) has lower state than invite
		);

		// both memberships match the identity but have different states
		assert_eq!(
			sort_membership(Some(&identity1), &membership1, &membership4),
			Ordering::Greater // active has lower state than join
		);

		// neither membership matches the identity but they have different states
		assert_eq!(
			sort_membership(Some(&identity3), &membership1, &membership2),
			Ordering::Greater // active has lower state than inactive regardless of identity match
		);
	}
}
