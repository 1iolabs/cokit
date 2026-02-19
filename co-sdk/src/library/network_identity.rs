// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use super::shared_membership::shared_membership;
use crate::{CoContext, CoReducer, CoReducerFactory};
use co_identity::{PrivateIdentityBox, PrivateIdentityResolver};
use co_primitives::{CoId, Did};

/// Network identity to sign messages to send into the network.
///
/// Currently this looks for the first membership found.
pub async fn network_identity(
	context: &CoContext,
	co: &CoReducer,
	prefered: Option<&Did>,
) -> Result<PrivateIdentityBox, anyhow::Error> {
	let parent_co_id = co.parent_id().ok_or(anyhow::anyhow!("no parent"))?;
	network_identity_by_id(context, parent_co_id, co.id(), prefered).await
}

/// Network identity to sign messages to send into the network.
///
/// Currently this looks for the first membership found.
pub async fn network_identity_by_id(
	context: &CoContext,
	parent_co_id: &CoId,
	co_id: &CoId,
	prefered: Option<&Did>,
) -> Result<PrivateIdentityBox, anyhow::Error> {
	let parent_co = context.try_co_reducer(parent_co_id).await?;
	let identity_did = shared_membership(&parent_co, co_id, prefered)
		.await?
		.ok_or(anyhow::anyhow!("No active membership"))?
		.did;
	let identity = context
		.private_identity_resolver()
		.await?
		.resolve_private(&identity_did)
		.await?;
	Ok(identity)
}
