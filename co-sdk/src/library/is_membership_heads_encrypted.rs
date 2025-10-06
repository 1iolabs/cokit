use crate::{is_cid_encrypted, CoStorage};
use co_core_membership::Membership;
use co_primitives::BlockStorageExt;

/// Test if any membership head is encrypted ([`co_primitives::KnownMultiCodec::CoEncryptedBlock`]).
pub async fn is_membership_heads_encrypted(
	storage: &CoStorage,
	membership: &Membership,
) -> Result<bool, anyhow::Error> {
	for co_state in membership.state.iter() {
		let (_state, heads) = storage.get_value(&co_state.state).await?.into_value();
		return Ok(is_cid_encrypted(&heads));
	}
	Ok(false)
}
