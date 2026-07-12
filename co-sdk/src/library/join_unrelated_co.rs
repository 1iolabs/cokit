// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{CoContext, CO_CORE_NAME_MEMBERSHIP};
use co_core_membership::{MembershipOptions, MembershipsAction};
use co_identity::{Identity, IdentityBox, PrivateIdentityBox};
use co_primitives::{tags, CoConnectivity, CoId, CoInviteMetadata, KnownTags};
use co_storage::BlockStorageExt;

/// Add a membership to a CO which are not participant of.
pub async fn join_unrelated_co(
	context: &CoContext,
	from: &PrivateIdentityBox,
	to: &IdentityBox,
	to_co: CoId,
	to_networks: impl Into<CoConnectivity>,
) -> Result<(), anyhow::Error> {
	let local_co = context.local_co_reducer().await?;

	// make sure connectivity contains at least `to` DID
	//  we need at least one pointer who to connect for unrelated COs
	let mut to_networks = to_networks.into();
	to_networks.participants.insert(to.identity().to_owned());

	// add membership
	let metadata = CoInviteMetadata {
		id: "unrelated".to_string(),
		from: to.identity().to_owned(),
		peer: None,
		network: to_networks,
		name: Default::default(),
		tags: Default::default(),
	};
	local_co
		.push(
			from,
			CO_CORE_NAME_MEMBERSHIP,
			&MembershipsAction::JoinPending {
				id: to_co,
				did: from.identity().to_owned(),
				options: MembershipOptions::default().with_tags(tags!(
					"owner": to.identity(),
					{KnownTags::CoInviteMetadata}: local_co.storage().set_serialized(&metadata).await?,
				)),
			},
		)
		.await?;

	// result
	Ok(())
}
