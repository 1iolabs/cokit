use crate::{
	state::{query_core, QueryExt},
	CoReducer, CO_CORE_NAME_STORAGE,
};
use co_core_storage::StorageAction;
use co_identity::PrivateIdentityBox;
use co_primitives::WeakCid;
use co_storage::BlockStorage;
use futures::{StreamExt, TryStreamExt};
use std::collections::BTreeSet;

/// Cleanup storage by removing all unreferenced blocks.
pub async fn storage_cleanup(identity: PrivateIdentityBox, reducer: &CoReducer) -> Result<usize, anyhow::Error> {
	let mut removed = 0;
	let mut query_blocks_index_unreferenced = query_core::<co_core_storage::Storage>(CO_CORE_NAME_STORAGE)
		.with_default()
		.map(|storage_core| storage_core.blocks_index_unreferenced);
	let mut query_blocks = query_core::<co_core_storage::Storage>(CO_CORE_NAME_STORAGE)
		.with_default()
		.map(|storage_core| storage_core.blocks);
	loop {
		// get chunk of blocks to remove
		let (storage, blocks_index_unreferenced) = query_blocks_index_unreferenced.execute_reducer(reducer).await?;
		let remove = blocks_index_unreferenced
			.stream(&storage)
			.take(256)
			.try_collect::<BTreeSet<WeakCid>>()
			.await?;
		if remove.is_empty() {
			break;
		}

		// remove from storage core
		reducer
			.push(&identity, CO_CORE_NAME_STORAGE, &StorageAction::Remove(remove.clone(), false))
			.await?;

		// remove from disk
		//  we double check if it has been removed because we dont use the force flag
		let mut last_error = Ok(());
		let (_, blocks) = query_blocks.execute_reducer(reducer).await?;
		let blocks = blocks.open(&storage).await?;
		for cid in remove {
			if !blocks.contains_key(&cid).await? {
				match storage.remove(&cid.cid()).await {
					Ok(_) => {
						removed += 1;
					},
					Err(err) => {
						// we only keep the last error and continue to minimize the risk of dead references in storage
						last_error = Err(err);
					},
				}
			}
		}
		last_error?;
	}

	// result
	Ok(removed)
}
