// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use super::QueryError;
use crate::{
	state::{query_core, Query},
	CO_CORE_NAME_CO, CO_CORE_NAME_MEMBERSHIP,
};
use co_core_membership::MembershipState;
use co_identity::{Identity, LocalIdentity};
use co_primitives::{CoId, Did, OptionLink, Tags};
use co_storage::BlockStorage;
use futures::Stream;

/// Returns memberships contained in the CO (`co_state`)`.
///
/// # Warning
/// - This will return all memberships and will not filter by [`co_core_membership::MembershipState`].
///
/// # Arguments
/// - `storage` - The BlockStorage.
/// - `co_state` - Co Core State (`co-core-co`).
pub fn memberships<S: BlockStorage + Clone + 'static>(
	storage: S,
	co_state: OptionLink<co_core_co::Co>,
) -> impl Stream<Item = Result<(CoId, Did, Tags, MembershipState), QueryError>> {
	async_stream::try_stream! {
		// root
		let co = query_core(CO_CORE_NAME_CO).with_default().execute(&storage, co_state).await?;
		if co_state.cid().is_some() {
			yield (co.id.clone(), LocalIdentity::device().identity().to_owned(), co.tags.clone(), MembershipState::Active);
		}

		// memberships
		let memberships = query_core(CO_CORE_NAME_MEMBERSHIP).with_default().execute(&storage, co_state).await?;
		let stream = memberships.memberships.stream(&storage);
		for await result in stream {
			let (_co_id, membership) = result?;
			for (did, membership_state) in &membership.did {
				yield (membership.id.clone(), did.clone(), membership.tags.clone(), *membership_state);
			}
		}
	}
}
