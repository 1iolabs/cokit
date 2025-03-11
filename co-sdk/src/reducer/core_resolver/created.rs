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
use co_storage::CreatedBlockStorage;
use std::collections::BTreeSet;

/// Reference count state in a [`co_core_storage::Storage`] core.
#[derive(Debug, Clone)]
pub struct CreatedCoreResolver<C> {
	next: C,
	storage: CreatedBlockStorage<CoStorage>,
}
impl<C> CreatedCoreResolver<C> {
	pub fn new(next: C, storage: CreatedBlockStorage<CoStorage>) -> Self {
		Self { next, storage }
	}
}
#[async_trait]
impl<S, C> CoreResolver<S> for CreatedCoreResolver<C>
where
	S: BlockStorage + Clone + Send + Sync + 'static,
	C: CoreResolver<S> + Clone + Send + Sync + 'static,
{
	#[tracing::instrument(skip(self, storage, runtime, state, action))]
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
		let dispatch = CoreResolverDispatch::<C, S, StorageAction>::new(
			self.next.clone(),
			runtime.clone(),
			context.clone(),
			storage.clone(),
			CO_CORE_NAME_STORAGE.to_owned(),
			next.state,
		);

		// flush changes
		let mut create_references = BTreeSet::new();
		for cid in self.storage.drain().await {
			// reference
			create_references.insert(cid);
			if create_references.len() > max_references {
				next.state = dispatch.dispatch(&StorageAction::ReferenceCreate(create_references)).await?;
				create_references = BTreeSet::new();
			}
		}
		if !create_references.is_empty() {
			next.state = dispatch.dispatch(&StorageAction::ReferenceCreate(create_references)).await?;
		}

		// result
		Ok(next)
	}
}
