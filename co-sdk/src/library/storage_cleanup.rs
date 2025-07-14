use super::max_reference_count::max_reference_count;
use crate::{
	state::{query_core, Query, QueryExt},
	types::co_dispatch::CoDispatch,
	StructureResolveResult, StructureResolver, CO_CORE_NAME_STORAGE,
};
use co_core_co::Co;
use co_core_storage::{BlockInfo, StorageAction};
use co_primitives::{OptionLink, StoreParams, WeakCid};
use co_storage::{BlockStorageContentMapping, ExtendedBlockStorage};
use futures::{pin_mut, TryStreamExt};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// Cleanup storage by removing all unreferenced blocks.
pub async fn storage_cleanup<S, D>(
	storage_core_storage: &S,
	storage_core_dispatcher: &mut impl CoDispatch<StorageAction>,
	storage_core_state: OptionLink<Co>,
	storage: &D,
	structure_resolver: &mut impl StructureResolver<S, D>,
) -> Result<(OptionLink<Co>, usize), anyhow::Error>
where
	S: ExtendedBlockStorage + BlockStorageContentMapping + Clone + 'static,
	D: ExtendedBlockStorage + BlockStorageContentMapping + Clone + 'static,
{
	let max_references = max_reference_count(<S::StoreParams as StoreParams>::MAX_BLOCK_SIZE);
	let mut removed_blocks = 0;
	let mut query_blocks_index_unreferenced = query_core(CO_CORE_NAME_STORAGE)
		.with_default()
		.map(|storage_core| storage_core.blocks_index_unreferenced);
	let mut query_blocks = query_core(CO_CORE_NAME_STORAGE)
		.with_default()
		.map(|storage_core| storage_core.blocks);
	let mut state = storage_core_state;
	loop {
		// open stream
		let blocks_index_unreferenced = query_blocks_index_unreferenced.execute(storage_core_storage, state).await?;
		let remove_stream = blocks_index_unreferenced.stream(storage_core_storage);
		pin_mut!(remove_stream);

		// group by info and resolve links
		let mut references_count = 0;
		let mut remove_from_disk = HashSet::<WeakCid>::new();
		let mut remove_by_info = HashMap::<BlockInfo, BTreeMap<WeakCid, BTreeSet<WeakCid>>>::new();
		while let Some((cid, info)) = remove_stream.try_next().await? {
			// filter
			let block_links = match structure_resolver.resolve(storage_core_storage, &info, storage, &cid).await? {
				StructureResolveResult::Exclude => {
					continue;
				},
				StructureResolveResult::Include(block_links) => block_links,
			};

			// add
			let by_info = remove_by_info.entry(info.clone()).or_insert(Default::default());
			if !by_info.contains_key(&cid) {
				let exists = storage.exists(&cid).await?;
				if exists {
					remove_from_disk.insert(cid);
				}

				// links
				let links = if exists && block_links.has_links(cid.cid()) {
					let block = storage.get(&cid).await?;
					let links = block_links.links(&block)?;
					links.map(WeakCid::from).collect::<BTreeSet<WeakCid>>()
				} else {
					Default::default()
				};

				// count
				references_count += 1 + links.len();
				if references_count > max_references {
					break;
				}

				// insert
				by_info.insert(cid, links);
			}
		}

		// done?
		if remove_by_info.is_empty() {
			break;
		}

		// remove from storage core
		for (info, delete) in remove_by_info {
			state = storage_core_dispatcher
				.dispatch(&StorageAction::Delete(info, delete, false))
				.await?
				.into();
		}

		// remove from disk
		//  we double check if it has been removed because we dont use the force flag
		let mut last_error = Ok(());
		let blocks = query_blocks.execute(storage_core_storage, state.into()).await?;
		let blocks = blocks.open(storage_core_storage).await?;
		for cid in remove_from_disk {
			let exists_in_core = blocks.contains_key(&cid).await?;
			#[cfg(feature = "logging-verbose")]
			tracing::trace!(?cid, ?exists_in_core, "storage-free-delete");
			if !exists_in_core {
				match storage_core_storage.remove(&cid.cid()).await {
					Ok(_) => {
						removed_blocks += 1;
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

	// log
	if removed_blocks > 0 {
		tracing::info!(removed_blocks, "storage-free");
	}

	// result
	Ok((state, removed_blocks))
}

#[cfg(test)]
mod tests {
	use crate::{
		state::{query_core, Query},
		types::co_pinning_key::CoPinningKey,
		ApplicationBuilder, CoReducer, MonotonicCoDate, MonotonicCoUuid, CO_CORE_NAME_CO, CO_CORE_NAME_STORAGE,
	};
	use co_core_co::CoAction;
	use co_core_storage::{PinStrategy, StorageAction};
	use co_primitives::tags;
	use co_storage::{ExtendedBlockStorage, TmpDir};
	use futures::TryStreamExt;

	async fn count_pin_references(co: &CoReducer, pin: CoPinningKey) -> u32 {
		let storage = co.storage();
		let co_state = co.reducer_state().await;
		let storage_core = query_core(CO_CORE_NAME_STORAGE)
			.execute(&storage, co_state.state().into())
			.await
			.unwrap();
		let pin = storage_core.pins.get(&storage, &pin.to_string(co.id())).await.unwrap().unwrap();
		let pin_references = pin.references.stream(&storage).try_collect::<Vec<_>>().await.unwrap();
		assert_eq!(pin_references.len(), pin.references_count as usize);
		let blocks_index_unreferenced = storage_core
			.blocks_index_unreferenced
			.stream(&storage)
			.try_collect::<Vec<_>>()
			.await
			.unwrap();
		tracing::trace!(?blocks_index_unreferenced, ?pin, ?pin_references, ?co_state, "test-check");

		// get count
		pin.references_count
	}

	/// Integration Test to verify storage_cleanup actualy deletes states.
	/// Note: The pinned state is always one state late.
	#[tokio::test]
	async fn integration_test_storage_cleanup() {
		let application_identifier = format!("integration_test_storage_cleanup-{}", uuid::Uuid::new_v4().to_string());
		let tmp = TmpDir::new("co");
		let application = ApplicationBuilder::new_with_path(application_identifier, tmp.path().to_owned())
			// .with_bunyan_logging(Some(std::env::current_dir().unwrap().join("../data/log/co.log")))
			.with_bunyan_logging(None)
			.with_disabled_feature("co-local-encryption")
			.with_setting("feature", "co-storage-free")
			.with_co_date(MonotonicCoDate::default())
			.with_co_uuid(MonotonicCoUuid::default())
			.without_keychain()
			.build()
			.await
			.unwrap();
		let local_co = application.local_co_reducer().await.unwrap();
		let storage = local_co.storage();
		assert_eq!(count_pin_references(&local_co, CoPinningKey::State).await, 1); // this contains the intermediate point before pinning

		// push
		let local_co_state = local_co.reducer_state().await;
		let external_local_co_state = local_co_state.to_external_force(&storage).await.unwrap();
		tracing::trace!(?local_co_state, ?external_local_co_state, "test-state");
		local_co
			.push(
				&application.local_identity(),
				CO_CORE_NAME_CO,
				&CoAction::TagsInsert { tags: tags!("hello": "world") },
			)
			.await
			.unwrap();
		assert_eq!(count_pin_references(&local_co, CoPinningKey::State).await, 3); // this contains the intermediate point before pinning, the actual state before and the next intermediate point.

		// only keep latest
		local_co
			.push(
				&application.local_identity(),
				CO_CORE_NAME_STORAGE,
				&StorageAction::PinUpdate(CoPinningKey::State.to_string(local_co.id()), PinStrategy::MaxCount(1)),
			)
			.await
			.unwrap();
		assert_eq!(count_pin_references(&local_co, CoPinningKey::State).await, 1);

		// push
		//  this will trigger the cleanup as the previous has set to one we not got items to remove
		local_co
			.push(&application.local_identity(), CO_CORE_NAME_CO, &CoAction::TagsInsert { tags: tags!("test": 123) })
			.await
			.unwrap();
		let next_local_co_state = local_co.reducer_state().await;
		let external_next_local_co_state = next_local_co_state.to_external_force(&storage).await.unwrap();
		tracing::trace!(?next_local_co_state, ?external_next_local_co_state, "test-state-next");
		assert_eq!(count_pin_references(&local_co, CoPinningKey::State).await, 1);

		// verify states are removed
		assert_eq!(storage.exists(&external_local_co_state.state().unwrap()).await.unwrap(), false);
		assert_eq!(storage.exists(&external_next_local_co_state.state().unwrap()).await.unwrap(), true);
		assert_eq!(storage.exists(&local_co_state.state().unwrap()).await.unwrap(), false);
		assert_eq!(storage.exists(&next_local_co_state.state().unwrap()).await.unwrap(), true);
	}
}
