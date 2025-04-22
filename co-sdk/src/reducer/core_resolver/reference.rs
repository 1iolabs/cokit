use super::{CoreResolver, CoreResolverError};
use crate::{
	library::{core_resolver_dispatch::CoreResolverDispatch, max_reference_count::max_reference_count},
	reducer::change::reference_writer::write_storage_references,
	types::co_dispatch::CoDispatch,
	CoStorage, ReducerChangeContext, Storage, CO_CORE_NAME_STORAGE,
};
use async_trait::async_trait;
use cid::Cid;
use co_actor::TaskSpawner;
use co_core_storage::StorageAction;
use co_primitives::{BlockLinks, BlockStorage, BlockStorageSettings, CloneWithBlockStorageSettings, StoreParams};
use co_runtime::{RuntimeContext, RuntimePool};
use co_storage::{Algorithm, EncryptedBlockStorage, ExtendedBlockStorage, OverlayBlockStorage, OverlayChange};
use futures::{pin_mut, TryStreamExt};
use std::{collections::BTreeSet, mem::swap};
use tracing::Instrument;

/// Reference count state in a [`co_core_storage::Storage`] core.
#[derive(Debug, Clone)]
pub struct ReferenceCoreResolver<C> {
	next: C,
	tasks: TaskSpawner,
	storage: Storage,
	pinning_key: Option<String>,
	block_links: BlockLinks,
}
impl<C> ReferenceCoreResolver<C> {
	pub fn new(next: C, tasks: TaskSpawner, storage: Storage, pinning_key: Option<String>) -> Self {
		Self { next, tasks, storage, pinning_key, block_links: Default::default() }
	}
}
impl<C> ReferenceCoreResolver<C>
where
	C: CoreResolver<CoStorage> + Clone + Send + Sync + 'static,
{
	async fn execute_with_tmp_storage(
		&self,
		tmp_storage: &CoStorage,
		storage: &CoStorage,
		runtime: &RuntimePool,
		context: &ReducerChangeContext,
		state: &Option<Cid>,
		action: &Cid,
	) -> Result<RuntimeContext, CoreResolverError> {
		let max_block_size = <<CoStorage as BlockStorage>::StoreParams as StoreParams>::MAX_BLOCK_SIZE;

		// transaction storage
		let overlay_storage = OverlayBlockStorage::new(
			self.tasks.clone(),
			storage.clone(),
			tmp_storage.clone(),
			max_block_size * 48,
			true,
		);
		let transaction_storage = CoStorage::new(overlay_storage.clone());

		// execute
		let mut next = self.next.execute(&transaction_storage, runtime, context, state, action).await?;

		// references
		if let Some(next_state) = next.state {
			// create storage core dispatcher
			let dispatch = CoreResolverDispatch::new(
				self.next.clone(),
				runtime.clone(),
				context.clone(),
				storage.clone(),
				CO_CORE_NAME_STORAGE.to_owned(),
				next.state,
			);

			// flush `next_state` from `overlay_storage` to `storage`.
			overlay_storage
				.flush(next_state, Some(self.block_links.clone()))
				.instrument(tracing::info_span!("overlay-flush"))
				.await?;

			// flush removed blocks from `overlay_storage` to `storage`.
			let max_references = max_reference_count(max_block_size);
			let mut remove = BTreeSet::new();
			let changes = overlay_storage.changes();
			pin_mut!(changes);
			while let Some(change) = changes.try_next().await? {
				match change {
					OverlayChange::Set(cid, _, _) => {
						// ignore as we only want referenced blocks
						tracing::warn!(?cid, ?action, "unreference-block");
					},
					OverlayChange::Remove(cid) => {
						storage.remove(&cid).await?;
						remove.insert(cid.into());

						// flush
						if remove.len() > max_references {
							let mut next_remove = Default::default();
							swap(&mut remove, &mut next_remove);
							let action = StorageAction::Remove(next_remove, true);
							next.state = dispatch.dispatch(&action).await?;
						}
					},
				}
			}
			if !remove.is_empty() {
				let action = StorageAction::Remove(remove, true);
				next.state = dispatch.dispatch(&action).await?;
			}

			// write references
			let local_overlay_storage =
				overlay_storage.clone_with_settings(BlockStorageSettings::new().without_networking());
			let local_transaction_storage = CoStorage::new(local_overlay_storage);
			next.state = write_storage_references(
				local_transaction_storage,
				&dispatch,
				self.block_links.clone(),
				self.pinning_key.clone(),
				*state,
				next_state,
			)
			.await?;
		}

		// result
		Ok(next)
	}
}
#[async_trait]
impl<C> CoreResolver<CoStorage> for ReferenceCoreResolver<C>
where
	C: CoreResolver<CoStorage> + Clone + Send + Sync + 'static,
{
	#[tracing::instrument(level = tracing::Level::TRACE, skip(self, storage, runtime, state, action))]
	async fn execute(
		&self,
		storage: &CoStorage,
		runtime: &RuntimePool,
		context: &ReducerChangeContext,
		state: &Option<Cid>,
		action: &Cid,
	) -> Result<RuntimeContext, CoreResolverError> {
		// transaction storage
		let tmp_storage = self.storage.tmp_storage();
		let algorithm = Algorithm::default();
		let encrypted_tmp_storage = CoStorage::new(EncryptedBlockStorage::new(
			tmp_storage,
			algorithm.generate_serect(),
			algorithm,
			Default::default(),
		));

		// execute
		let result = self
			.execute_with_tmp_storage(&encrypted_tmp_storage, storage, runtime, context, state, action)
			.await;

		// cleanup
		encrypted_tmp_storage.clear().await?;

		// result
		Ok(result?)
	}
}
