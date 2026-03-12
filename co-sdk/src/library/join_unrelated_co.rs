use crate::{CoContext, CO_CORE_NAME_MEMBERSHIP};
use co_core_membership::{Membership, MembershipState, MembershipsAction};
use co_identity::{Identity, IdentityBox, PrivateIdentityBox};
use co_primitives::{tags, CoConnectivity, CoId, CoInviteMetadata, KnownTags, Network};
use co_storage::BlockStorageExt;
use std::collections::BTreeSet;

/// Add a membership to a CO which are not participant of.
pub async fn join_unrelated_co(
	context: &CoContext,
	from: &PrivateIdentityBox,
	to: &IdentityBox,
	to_co: CoId,
	to_networks: BTreeSet<Network>,
) -> Result<(), anyhow::Error> {
	let local_co = context.local_co_reducer().await?;

	// add membership
	let metadata = CoInviteMetadata {
		id: "unrelated".to_string(),
		from: to.identity().to_owned(),
		peer: None,
		network: CoConnectivity { network: to_networks, participants: Default::default() },
	};
	let membership = Membership {
		id: to_co,
		did: from.identity().to_owned(),
		state: Default::default(),
		key: None,
		membership_state: MembershipState::Pending,
		tags: tags!(
			"owner": to.identity(),
			{KnownTags::CoInviteMetadata}: local_co.storage().set_serialized(&metadata).await?,
		),
	};
	local_co
		.push(from, CO_CORE_NAME_MEMBERSHIP, &MembershipsAction::Join(membership))
		.await?;

	// result
	Ok(())
}
