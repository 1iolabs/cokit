// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use cid::Cid;
use co_core_membership::CoState;
use co_storage::{BlockStorage, BlockStorageExt, StorageError};
use std::collections::BTreeSet;

/// Resolve states to heads.
pub async fn membership_all_heads<S: BlockStorage + 'static>(
	storage: &S,
	states: impl Iterator<Item = &CoState>,
) -> Result<BTreeSet<Cid>, StorageError> {
	let mut remove = BTreeSet::new();
	for state in states {
		let reference = storage.get_value(&state.state).await?;
		let heads = reference.into_value().1;
		remove.extend(heads);
	}
	Ok(remove)
}
