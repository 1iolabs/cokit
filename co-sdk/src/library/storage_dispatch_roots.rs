// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{
	library::{storage_structure::STORAGE_CO_ROOT_TYPE, to_external_cid::to_external_cid},
	types::co_dispatch::CoDispatch,
	CoPinningKey, CoReducerState, CoRoot,
};
use co_core_storage::{References, StorageAction};
use co_primitives::{tags, CoId};
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
	let mut roots = References::new();
	for reducer_state in co_new_roots.iter() {
		let co_root = CoRoot::from(reducer_state.clone());
		let co_root_reference = co_storage.set_serialized(&co_root).await?;
		let external_co_root_reference = to_external_cid(co_storage, co_root_reference).await;
		roots.insert_with_tags(external_co_root_reference, tags!("type": STORAGE_CO_ROOT_TYPE));
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
