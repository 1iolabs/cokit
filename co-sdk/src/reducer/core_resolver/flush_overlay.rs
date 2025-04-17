use super::{CoreResolver, CoreResolverError};
use crate::{
	library::{core_resolver_dispatch::CoreResolverDispatch, max_reference_count::max_reference_count},
	types::co_dispatch::CoDispatch,
	CoStorage, ReducerChangeContext, CO_CORE_NAME_STORAGE,
};
use async_trait::async_trait;
use cid::Cid;
use co_core_storage::StorageAction;
use co_primitives::{BlockStorage, StoreParams};
use co_runtime::{RuntimeContext, RuntimePool};
use co_storage::{OverlayBlockStorage, OverlayChangeReference};
use futures::{pin_mut, TryStreamExt};
use std::collections::BTreeSet;

/// Reference count state in a [`co_core_storage::Storage`] core.
#[derive(Debug, Clone)]
pub struct FlushOverlayCoreResolver<C> {
	next: C,
	overlay_storage: OverlayBlockStorage<CoStorage>,
}
impl<C> FlushOverlayCoreResolver<C> {
	pub fn new(next: C, overlay_storage: OverlayBlockStorage<CoStorage>) -> Self {
		Self { next, overlay_storage }
	}
}
#[async_trait]
impl<S, C> CoreResolver<S> for FlushOverlayCoreResolver<C>
where
	S: BlockStorage + Clone + Send + Sync + 'static,
	C: CoreResolver<S> + Clone + Send + Sync + 'static,
{
	#[tracing::instrument(level = tracing::Level::TRACE, skip(self, storage, runtime, state, action))]
	async fn execute(
		&self,
		storage: &S,
		runtime: &RuntimePool,
		context: &ReducerChangeContext,
		state: &Option<Cid>,
		action: &Cid,
	) -> Result<RuntimeContext, CoreResolverError> {
		// execute
		let mut next = self.next.execute(storage, runtime, context, state, action).await?;

		// calc max references per action
		let max_references = max_reference_count(S::StoreParams::MAX_BLOCK_SIZE);

		// create storage core dispatcher
		let dispatch = CoreResolverDispatch::new(
			self.next.clone(),
			runtime.clone(),
			context.clone(),
			storage.clone(),
			CO_CORE_NAME_STORAGE.to_owned(),
			next.state,
		);

		// flush changes
		let mut create_references = BTreeSet::new();
		let changes = self.overlay_storage.flush_changes();
		pin_mut!(changes);
		while let Some(change) = changes.try_next().await? {
			match change {
				OverlayChangeReference::Set(cid) => {
					// reference
					create_references.insert(cid.into());
					if create_references.len() > max_references {
						next.state = dispatch.dispatch(&StorageAction::ReferenceCreate(create_references)).await?;
						create_references = BTreeSet::new();
					}
				},
				OverlayChangeReference::Remove(_) => {
					// ignore
				},
			}
		}
		if !create_references.is_empty() {
			next.state = dispatch.dispatch(&StorageAction::ReferenceCreate(create_references)).await?;
		}

		// flush storage core blocks
		let changes = self.overlay_storage.flush_changes();
		pin_mut!(changes);
		while let Some(_change) = changes.try_next().await? {
			// ignore
		}

		// result
		Ok(next)
	}
}
