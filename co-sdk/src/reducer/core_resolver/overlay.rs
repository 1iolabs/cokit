use super::{CoreResolver, CoreResolverError};
#[cfg(feature = "pinning")]
use crate::library::max_reference_count::max_reference_count;
#[cfg(feature = "pinning")]
use crate::types::co_dispatch::CoDispatch;
#[cfg(feature = "pinning")]
use crate::{library::core_resolver_dispatch::CoreResolverDispatch, CO_CORE_NAME_STORAGE};
use crate::{CoStorage, ReducerChangeContext, Storage};
use async_trait::async_trait;
use cid::Cid;
use co_actor::TaskSpawner;
#[cfg(feature = "pinning")]
use co_core_storage::StorageAction;
use co_primitives::{BlockLinks, BlockStorage};
#[cfg(feature = "pinning")]
use co_primitives::{StoreParams, WeakCid};
use co_runtime::{RuntimeContext, RuntimePool};
use co_storage::{ExtendedBlockStorage, OverlayBlockStorage, OverlayChange};
use futures::{pin_mut, TryStreamExt};
#[cfg(feature = "pinning")]
use std::collections::BTreeSet;
use tracing::Instrument;

/// Write to overlay state to isolate storage changes.
#[derive(Debug, Clone)]
pub struct OverlayCoreResolver<C> {
	next: C,
	tasks: TaskSpawner,
	storage: Storage,
	block_links: BlockLinks,
}
impl<C> OverlayCoreResolver<C> {
	pub fn new(next: C, tasks: TaskSpawner, storage: Storage) -> Self {
		Self { next, tasks, storage, block_links: Default::default() }
	}
}
impl<C> OverlayCoreResolver<C>
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
		let overlay_storage =
			OverlayBlockStorage::new(self.tasks.clone(), storage.clone(), tmp_storage.clone(), None, true, false);
		let transaction_storage = CoStorage::new(overlay_storage.clone());

		// execute
		let mut next = self.next.execute(&transaction_storage, runtime, context, state, action).await?;

		// references
		if let Some(next_state) = next.state {
			// flush `next_state` from `overlay_storage` to `storage`.
			overlay_storage
				.flush(next_state, Some(self.block_links.clone()))
				.instrument(tracing::info_span!("overlay-flush-state", cid = ?next_state))
				.await?;

			// resolve diagnostics
			next.resolve_diagnostics(&overlay_storage).await?;

			// flush removed blocks from `overlay_storage` to `storage`.
			#[cfg(feature = "pinning")]
			let mut dispatch = CoreResolverDispatch::new(
				self.next.clone(),
				runtime.clone(),
				context.clone(),
				storage.clone(),
				CO_CORE_NAME_STORAGE.to_owned(),
				next.state,
			);
			#[cfg(feature = "pinning")]
			let max_references = max_reference_count(<<CoStorage as BlockStorage>::StoreParams as StoreParams>::MAX_BLOCK_SIZE);
			#[cfg(feature = "pinning")]
			let mut remove = BTreeSet::<WeakCid>::new();
			let changes = overlay_storage.changes();
			pin_mut!(changes);
			while let Some(change) = changes.try_next().await? {
				match change {
					OverlayChange::Set(_cid, _data, _) => {
						// ignore as we only want referenced blocks
						//  this is not "bad" it just indicates that some block got stored which are not used
						//  this also could be intermediate computation inside a core that has later been overwritten

						// log
						#[cfg(feature = "logging-verbose")]
						if co_primitives::MultiCodec::is_cbor(_cid) {
							tracing::warn!(cid = ?_cid, ?action, ipld = ?co_primitives::from_cbor::<ipld_core::ipld::Ipld>(&_data), "overlay-unreferenced-block");
						} else {
							tracing::warn!(cid = ?_cid, ?action, "overlay-unreferenced-block");
						}
					},
					OverlayChange::Remove(cid) => {
						// remove
						storage.remove(&cid).await?;

						// flush
						#[cfg(feature = "pinning")]
						{
							remove.insert(cid.into());
							if remove.len() > max_references {
								let mut next_remove = Default::default();
								std::mem::swap(&mut remove, &mut next_remove);
								let action = StorageAction::Remove(next_remove, true);
								next.state = dispatch.dispatch(&action).await?;
							}
						}
					},
				}
			}
			#[cfg(feature = "pinning")]
			if !remove.is_empty() {
				let action = StorageAction::Remove(remove, true);
				next.state = dispatch.dispatch(&action).await?;
			}
		}

		// result
		Ok(next)
	}
}
#[async_trait]
impl<C> CoreResolver<CoStorage> for OverlayCoreResolver<C>
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
