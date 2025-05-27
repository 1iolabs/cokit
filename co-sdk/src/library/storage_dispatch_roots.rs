use crate::{types::co_dispatch::CoDispatch, CoPinningKey, CoReducerState};
use co_core_storage::StorageAction;
use co_primitives::{CoId, WeakCid};
use co_storage::{BlockStorage, BlockStorageContentMapping};
use indexmap::IndexSet;

/// Apply root pins to storage core.
pub async fn storage_dispatch_roots<S>(
	storage: &S,
	dispatch: &mut impl CoDispatch<StorageAction>,
	pin: &CoId,
	new_roots: Vec<CoReducerState>,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + BlockStorageContentMapping + 'static,
{
	// collect
	let mut states = IndexSet::new();
	let mut heads = IndexSet::new();
	for root in &new_roots {
		let external_root = root.to_external(storage).await;
		if let Some(state) = external_root.state() {
			states.insert(state);
		}
		for head in external_root.heads() {
			heads.insert(head);
		}
	}

	// log
	#[cfg(feature = "logging-verbose")]
	tracing::trace!(co = ?pin, ?new_roots, ?states, ?heads, "storage-roots");

	// insert heads
	if !heads.is_empty() {
		let action = StorageAction::PinReference(
			CoPinningKey::Log.to_string(pin),
			heads.into_iter().map(WeakCid::from).collect(),
		);
		dispatch.dispatch(&action).await?;
	}

	// insert states
	if !states.is_empty() {
		let action = StorageAction::PinReference(
			CoPinningKey::State.to_string(pin),
			states.into_iter().map(WeakCid::from).collect(),
		);
		dispatch.dispatch(&action).await?;
	}

	// result
	Ok(())
}
