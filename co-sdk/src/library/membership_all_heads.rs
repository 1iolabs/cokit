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
