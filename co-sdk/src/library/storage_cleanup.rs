use crate::{
	library::{max_reference_count::max_reference_count, to_internal_cid::to_internal_mapped},
	state::{query_core, Query, QueryExt},
	types::co_dispatch::CoDispatch,
	StructureResolveResult, StructureResolver, CO_CORE_NAME_STORAGE,
};
use co_core_co::Co;
use co_core_storage::{BlockInfo, StorageAction};
use co_primitives::{OptionLink, StoreParams, WeakCid};
use co_storage::{BlockStorageContentMapping, ExtendedBlockStorage};
use futures::{pin_mut, TryStreamExt};
use std::collections::{btree_map::Entry, BTreeMap, BTreeSet, HashMap, HashSet};

/// Cleanup storage by removing all unreferenced blocks.
#[tracing::instrument(level = tracing::Level::TRACE, name = "storage-cleanup", err(Debug), skip_all)]
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
		// open blocks
		let blocks = query_blocks.execute(storage_core_storage, state).await?;
		let blocks = blocks.open(storage_core_storage).await?;

		// open stream
		let blocks_index_unreferenced = query_blocks_index_unreferenced.execute(storage_core_storage, state).await?;
		let remove_stream = blocks_index_unreferenced.stream(storage_core_storage);
		pin_mut!(remove_stream);

		// group by info and resolve links
		let mut references_count = 0;
		let mut remove_from_disk = HashSet::<WeakCid>::new();
		let mut remove_by_info = HashMap::<BlockInfo, BTreeMap<WeakCid, BTreeSet<WeakCid>>>::new();
		while let Some((cid, info)) = remove_stream.try_next().await? {
			// get tags
			let block_tags = match blocks.get(&cid).await? {
				Some(block) => block.tags.clone(),
				None => Default::default(),
			};

			// map
			let Some(mapped_cid) = to_internal_mapped(storage, cid.into()).await else {
				continue;
			};

			// filter
			let external_links = match structure_resolver
				.resolve(storage_core_storage, &info, storage, &mapped_cid, &block_tags)
				.await?
			{
				StructureResolveResult::Exclude => {
					continue;
				},
				StructureResolveResult::Include(block_references) => block_references,
			};

			// add
			let by_info = remove_by_info.entry(info.clone()).or_default();
			if let Entry::Vacant(e) = by_info.entry(cid) {
				let exists = storage.exists(&cid).await?;
				if exists {
					remove_from_disk.insert(cid);
				}

				// insert
				e.insert(external_links.iter().collect());

				// count
				references_count += 1 + external_links.len();
				if references_count > max_references {
					break;
				}
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
		let blocks = query_blocks.execute(storage_core_storage, state).await?;
		let blocks = blocks.open(storage_core_storage).await?;
		for cid in remove_from_disk {
			let exists_in_core = blocks.contains_key(&cid).await?;
			#[cfg(feature = "logging-verbose")]
			tracing::trace!(?cid, ?exists_in_core, "storage-cleanup-delete");
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
		tracing::info!(removed_blocks, "storage-cleanup-free");
	}

	// result
	Ok((state, removed_blocks))
}

#[cfg(test)]
mod tests {
	use crate::{
		state::{query_core, Query},
		types::co_pinning_key::CoPinningKey,
		ApplicationBuilder, CoReducer, CreateCo, DidKeyProvider, MonotonicCoDate, MonotonicCoUuid, CO_CORE_NAME_CO,
		CO_CORE_NAME_KEYSTORE, CO_CORE_NAME_STORAGE,
	};
	use co_core_co::CoAction;
	use co_core_storage::{PinStrategy, StorageAction};
	use co_identity::DidKeyIdentity;
	use co_primitives::{tags, CoId};
	use co_storage::ExtendedBlockStorage;
	use co_test::{test_application_identifier, test_log_path, test_tmp_dir};
	use futures::TryStreamExt;

	async fn count_pin_references(local_co: &CoReducer, co: &CoId, pin: CoPinningKey) -> u32 {
		let storage = local_co.storage();
		let co_state = local_co.reducer_state().await;
		let storage_core = query_core(CO_CORE_NAME_STORAGE)
			.execute(&storage, co_state.state().into())
			.await
			.unwrap();
		let pin = storage_core.pins.get(&storage, &pin.to_string(co)).await.unwrap().unwrap();
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
	async fn integration_test_storage_cleanup_local() {
		let application_identifier = test_application_identifier("integration_test_storage_cleanup");
		let tmp = test_tmp_dir();
		let application = ApplicationBuilder::new_with_path(application_identifier, tmp.path().to_owned())
			.with_bunyan_logging(Some(test_log_path()))
			.with_optional_tracing()
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
		assert_eq!(count_pin_references(&local_co, local_co.id(), CoPinningKey::Root).await, 1); // this contains the intermediate point before pinning

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
		assert_eq!(count_pin_references(&local_co, local_co.id(), CoPinningKey::Root).await, 3); // this contains the intermediate point before pinning, the actual state before and the next intermediate point.

		// only keep latest
		local_co
			.push(
				&application.local_identity(),
				CO_CORE_NAME_STORAGE,
				&StorageAction::PinUpdate(CoPinningKey::Root.to_string(local_co.id()), PinStrategy::MaxCount(1)),
			)
			.await
			.unwrap();
		assert_eq!(count_pin_references(&local_co, local_co.id(), CoPinningKey::Root).await, 1);

		// push
		//  this will trigger the cleanup as the previous has set to one we not got items to remove
		local_co
			.push(&application.local_identity(), CO_CORE_NAME_CO, &CoAction::TagsInsert { tags: tags!("test": 123) })
			.await
			.unwrap();
		let next_local_co_state = local_co.reducer_state().await;
		let external_next_local_co_state = next_local_co_state.to_external_force(&storage).await.unwrap();
		tracing::trace!(?next_local_co_state, ?external_next_local_co_state, "test-state-next");
		assert_eq!(count_pin_references(&local_co, local_co.id(), CoPinningKey::Root).await, 1);

		// verify states are removed
		assert_eq!(storage.exists(&external_local_co_state.state().unwrap()).await.unwrap(), false);
		assert_eq!(storage.exists(&external_next_local_co_state.state().unwrap()).await.unwrap(), true);
		assert_eq!(storage.exists(&local_co_state.state().unwrap()).await.unwrap(), false);
		assert_eq!(storage.exists(&next_local_co_state.state().unwrap()).await.unwrap(), true);

		// verify heads are removed
		assert_eq!(storage.exists(external_local_co_state.heads().first().unwrap()).await.unwrap(), false);
		assert_eq!(
			storage
				.exists(external_next_local_co_state.heads().first().unwrap())
				.await
				.unwrap(),
			true
		);
		assert_eq!(storage.exists(local_co_state.heads().first().unwrap()).await.unwrap(), false);
		assert_eq!(storage.exists(next_local_co_state.heads().first().unwrap()).await.unwrap(), true);
	}

	/// Integration Test to verify storage_cleanup actualy deletes states.
	///
	/// # Note
	/// The pinned state is always one state late.
	///
	/// # Debug
	/// ```shell
	/// co --base-path <path> --no-keychain --no-default-features co log local
	/// ```
	#[tokio::test]
	async fn integration_test_storage_cleanup_shared() {
		let application_identifier = test_application_identifier("integration_test_storage_cleanup_shared");
		let tmp = test_tmp_dir();
		let application = ApplicationBuilder::new_with_path(application_identifier, tmp.path().to_owned())
			.with_bunyan_logging(Some(test_log_path()))
			.with_optional_tracing()
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

		// create identity
		let identity = DidKeyIdentity::generate(None);
		let co = application.local_co_reducer().await.unwrap();
		let provider = DidKeyProvider::new(co, CO_CORE_NAME_KEYSTORE);
		provider.store(&identity, None).await.unwrap();

		// create co
		let co = application
			.create_co(identity.clone(), CreateCo::new("shared", None).with_algorithm(None))
			.await
			.unwrap();

		// when the co has fully initialized check the initial state count
		let _co_state = co.reducer_state().await;
		assert_eq!(count_pin_references(&local_co, co.id(), CoPinningKey::Root).await, 1);

		// push
		let co_state = co.reducer_state().await;
		let external_co_state = co_state.to_external_force(&storage).await.unwrap();
		tracing::trace!(?co_state, ?external_co_state, "test-state");
		co.push(
			&application.local_identity(),
			CO_CORE_NAME_CO,
			&CoAction::TagsInsert { tags: tags!("hello": "world") },
		)
		.await
		.unwrap();
		assert_eq!(count_pin_references(&local_co, co.id(), CoPinningKey::Root).await, 2); // this contains the intermediate point before pinning, the actual state before and the next intermediate point.

		// only keep latest
		local_co
			.push(
				&application.local_identity(),
				CO_CORE_NAME_STORAGE,
				&StorageAction::PinUpdate(CoPinningKey::Root.to_string(co.id()), PinStrategy::MaxCount(1)),
			)
			.await
			.unwrap();
		assert_eq!(count_pin_references(&local_co, co.id(), CoPinningKey::Root).await, 1);

		// push
		//  this will trigger the cleanup as the previous has set to one we not got items to remove
		co.push(&application.local_identity(), CO_CORE_NAME_CO, &CoAction::TagsInsert { tags: tags!("test": 123) })
			.await
			.unwrap();
		let next_co_state = co.reducer_state().await;
		let external_next_co_state = next_co_state.to_external_force(&storage).await.unwrap();
		tracing::trace!(?next_co_state, ?external_next_co_state, "test-state-next");
		assert_eq!(count_pin_references(&local_co, co.id(), CoPinningKey::Root).await, 1);

		// verify states are removed
		assert_eq!(storage.exists(&external_co_state.state().unwrap()).await.unwrap(), false);
		assert_eq!(storage.exists(&external_next_co_state.state().unwrap()).await.unwrap(), true);
		assert_eq!(storage.exists(&co_state.state().unwrap()).await.unwrap(), false);
		assert_eq!(storage.exists(&next_co_state.state().unwrap()).await.unwrap(), true);

		// verify heads are removed
		assert_eq!(storage.exists(external_co_state.heads().first().unwrap()).await.unwrap(), false);
		assert_eq!(storage.exists(external_next_co_state.heads().first().unwrap()).await.unwrap(), true);
		assert_eq!(storage.exists(co_state.heads().first().unwrap()).await.unwrap(), false);
		assert_eq!(storage.exists(next_co_state.heads().first().unwrap()).await.unwrap(), true);
	}
}
