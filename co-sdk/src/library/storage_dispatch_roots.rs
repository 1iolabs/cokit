use crate::{
	library::to_external_cid::to_external_cid, types::co_dispatch::CoDispatch, CoPinningKey, CoReducerState, CoRoot,
};
use co_core_storage::StorageAction;
use co_primitives::{CoId, WeakCid};
use co_storage::{BlockStorage, BlockStorageContentMapping, BlockStorageExt};

/// Apply root pins to storage core.
pub async fn storage_dispatch_roots<S>(
	dispatch: &mut impl CoDispatch<StorageAction>,
	co_storage: &S,
	co: &CoId,
	co_new_roots: Vec<CoReducerState>,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + BlockStorageContentMapping + 'static,
{
	// store
	let mut roots = Vec::new();
	for reducer_state in co_new_roots.iter() {
		let co_root = CoRoot::from(reducer_state.clone());
		let co_root_reference = co_storage.set_serialized(&co_root).await?;
		let external_co_root_reference = to_external_cid(co_storage, co_root_reference).await;
		roots.push(WeakCid::from(external_co_root_reference));
	}

	// log
	#[cfg(feature = "logging-verbose")]
	tracing::trace!(?co, ?co_new_roots, ?roots, "storage-roots");

	// insert roots
	if !roots.is_empty() {
		let action = StorageAction::PinReference(CoPinningKey::Root.to_string(co), roots);
		dispatch.dispatch(&action).await?;
	}

	// result
	Ok(())
}
