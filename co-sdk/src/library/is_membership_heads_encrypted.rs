// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{is_cid_encrypted, CoStorage};
use co_core_membership::Membership;
use co_primitives::BlockStorageExt;

/// Test if any membership head is encrypted ([`co_primitives::KnownMultiCodec::CoEncryptedBlock`]).
pub async fn is_membership_heads_encrypted(
	storage: &CoStorage,
	membership: &Membership,
) -> Result<bool, anyhow::Error> {
	if let Some(co_state) = membership.state.iter().next() {
		let (_state, heads) = storage.get_value(&co_state.state).await?.into_value();
		return Ok(is_cid_encrypted(&heads));
	}
	Ok(false)
}
