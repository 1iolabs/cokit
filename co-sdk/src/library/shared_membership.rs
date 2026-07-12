// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{library::find_membership::find_membership_by, CoOptions, CoReducer};
use co_actor::time;
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
/// If `unknown` is `true` also wait if there is no membership yet.
pub async fn wait_shared_membership_active(
	parent: &CoReducer,
	co: &CoId,
	identity: Option<&Did>,
	unknown: bool,
) -> Result<Option<Membership>, anyhow::Error> {
	// decide whether to wait, based on the current membership state.
	let should_wait = match shared_membership(parent, co, identity).await? {
		Some(membership) => match membership.membership_state() {
			// already active — done.
			Some(MembershipState::Active) => return Ok(Some(membership)),
			// in-flight join - wait for it to become active.
			Some(MembershipState::Pending | MembershipState::Join) => true,
			// entry exists without a resolved state - wait only for unknown COs.
			None if unknown => true,
			_ => false,
		},
		// no membership entry yet
		None => unknown,
	};

	if !should_wait {
		return Ok(None);
	}

	// wait until a membership appears (if needed) and becomes active.
	parent
		.reducer_state_stream()
		.map(Ok)
		.try_filter_map(move |_parent_reducer_state| {
			let parent = parent.clone();
			let co = co.clone();
			let identity = identity.cloned();
			async move { shared_membership_active(&parent, &co, identity.as_ref()).await }
		})
		.try_first()
		.await
}

/// Find active shared membership with options.
pub async fn shared_membership_active_options(
	parent: &CoReducer,
	co: &CoId,
	identity: Option<&Did>,
	options: CoOptions,
) -> Result<Option<Membership>, anyhow::Error> {
	// find first active membership
	Ok(if options.wait || options.wait_unknown {
		if let Some(timeout) = options.wait_timeout {
			time::timeout(timeout, wait_shared_membership_active(parent, co, identity, options.wait_unknown)).await??
		} else {
			wait_shared_membership_active(parent, co, identity, options.wait_unknown).await?
		}
	} else {
		shared_membership_active(parent, co, identity).await?
	})
}
