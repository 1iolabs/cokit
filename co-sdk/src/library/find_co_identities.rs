// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{find_membership, library::network_identity::network_identity_by_id, CoContext, CO_ID_LOCAL};
use co_identity::PrivateIdentityBox;
use co_primitives::{CoId, Did};

pub async fn find_co_identities(context: &CoContext, co: &CoId) -> anyhow::Result<Vec<Did>> {
	let local_co = context.local_co_reducer().await?;
	let membership = find_membership(&local_co, co).await?;
	Ok(membership.into_iter().flat_map(|m| m.did.into_keys()).collect())
}

pub async fn find_co_private_identity(context: &CoContext, co: &CoId) -> anyhow::Result<PrivateIdentityBox> {
	network_identity_by_id(context, &CoId::from(CO_ID_LOCAL), co, None).await
}
