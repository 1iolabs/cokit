use super::{CoreResolver, CoreResolverError};
use crate::{
	library::core_resolver_dispatch::CoreResolverDispatch, reducer::change::reference_writer::write_storage_references,
	CoStorage, ReducerChangeContext, CO_CORE_NAME_STORAGE,
};
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{BlockLinks, BlockStorageSettings, CloneWithBlockStorageSettings};
use co_runtime::{RuntimeContext, RuntimePool};

/// Reference count state in a [`co_core_storage::Storage`] core.
#[derive(Debug, Clone)]
pub struct ReferenceCoreResolver<C> {
	next: C,
	pinning_key: Option<String>,
	block_links: BlockLinks,
}
impl<C> ReferenceCoreResolver<C> {
	pub fn new(next: C, pinning_key: Option<String>) -> Self {
		Self { next, pinning_key, block_links: Default::default() }
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
		// execute
		let mut next = self.next.execute(&storage, runtime, context, state, action).await?;

		// references
		if let Some(next_state) = next.state {
			let mut dispatch = CoreResolverDispatch::new(
				self.next.clone(),
				runtime.clone(),
				context.clone(),
				storage.clone(),
				CO_CORE_NAME_STORAGE.to_owned(),
				next.state,
			);
			next.state = write_storage_references(
				storage.clone_with_settings(BlockStorageSettings::new().without_networking()),
				&mut dispatch,
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
