use super::{CoreResolver, CoreResolverError};
use crate::{
	library::core_resolver_dispatch::CoreResolverDispatch, reducer::change::reference_writer::ReferenceWriter,
	types::co_reducer_context::CoReducerContextRef, ReducerChangeContext, CO_CORE_NAME_STORAGE,
};
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{BlockStorage, StoreParams};
use co_runtime::{RuntimeContext, RuntimePool};

/// Reference count state in a [`co_core_storage::Storage`] core.
#[derive(Debug, Clone)]
pub struct ReferenceCoreResolver<C> {
	next: C,
	pinning_key: Option<String>,
	reducer_context: CoReducerContextRef,
}
impl<C> ReferenceCoreResolver<C> {
	pub fn new(next: C, pinning_key: Option<String>, reducer_context: CoReducerContextRef) -> Self {
		Self { next, pinning_key, reducer_context }
	}
}
#[async_trait]
impl<S, C> CoreResolver<S> for ReferenceCoreResolver<C>
where
	S: BlockStorage + Send + Sync + Clone + 'static,
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

			// write references
			let reference_writer =
				ReferenceWriter::new(dispatch, self.reducer_context.clone(), self.pinning_key.clone());
			next.state = reference_writer
				.write(*state, next_state, <S::StoreParams as StoreParams>::MAX_BLOCK_SIZE)
				.await?;
		}

		// result
		Ok(next)
	}
}
