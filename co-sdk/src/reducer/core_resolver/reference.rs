use super::{CoreResolver, CoreResolverError};
use crate::{
	library::core_resolver_dispatch::CoreResolverDispatch, reducer::change::reference_writer::write_storage_references,
	CoStorage, ReducerChangeContext, Storage, CO_CORE_NAME_STORAGE,
};
use async_trait::async_trait;
use cid::Cid;
use co_actor::TaskSpawner;
use co_primitives::{BlockStorage, BlockStorageSettings, CloneWithBlockStorageSettings, StoreParams};
use co_runtime::{RuntimeContext, RuntimePool};
use co_storage::{ExtendedBlockStorage, OverlayBlockStorage};

/// Reference count state in a [`co_core_storage::Storage`] core.
#[derive(Debug, Clone)]
pub struct ReferenceCoreResolver<C> {
	next: C,
	tasks: TaskSpawner,
	storage: Storage,
	pinning_key: Option<String>,
}
impl<C> ReferenceCoreResolver<C> {
	pub fn new(next: C, tasks: TaskSpawner, storage: Storage, pinning_key: Option<String>) -> Self {
		Self { next, tasks, storage, pinning_key }
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
		// transaction storage
		let overlay_storage = OverlayBlockStorage::new(
			self.tasks.clone(),
			storage.clone(),
			tmp_storage.clone(),
			<<CoStorage as BlockStorage>::StoreParams as StoreParams>::MAX_BLOCK_SIZE * 48,
			true,
		);
		let transaction_storage = CoStorage::new(overlay_storage.clone());

		// execute
		let mut next = self.next.execute(&transaction_storage, runtime, context, state, action).await?;

		// references
		if let Some(next_state) = next.state {
			// create a transaction storage instance that flushes changes one-thy-fly while they got read
			let local_flush_overlay_storage = overlay_storage
				.clone_with_settings(BlockStorageSettings::new().without_networking())
				.with_flush_on_the_fly(true);
			let local_transaction_storage = CoStorage::new(local_flush_overlay_storage);

			// create storage core dispatcher
			let dispatch = CoreResolverDispatch::new(
				self.next.clone(),
				runtime.clone(),
				context.clone(),
				storage.clone(),
				CO_CORE_NAME_STORAGE.to_owned(),
				next.state,
			);

			// write references
			next.state = write_storage_references(
				local_transaction_storage,
				&dispatch,
				self.pinning_key.clone(),
				*state,
				next_state,
				<<CoStorage as BlockStorage>::StoreParams as StoreParams>::MAX_BLOCK_SIZE,
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
		// TODO (security): use encrypted storage for tmp files?
		let tmp_storage = self.storage.tmp_storage();

		// execute
		let result = self
			.execute_with_tmp_storage(&tmp_storage, storage, runtime, context, state, action)
			.await;

		// cleanup
		tmp_storage.clear().await?;

		// result
		Ok(result?)
	}
}
