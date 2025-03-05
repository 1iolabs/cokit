use super::{CoreResolver, CoreResolverError};
use crate::{
	library::runtime_dispatch::RuntimeDispatch,
	reducer::change::reference_writer::ReferenceWriter,
	types::{co_reducer::CoReducerContextRef, cores::CO_CORE_STORAGE},
	Cores, ReducerChangeContext, CO_CORE_NAME_STORAGE,
};
use async_trait::async_trait;
use cid::Cid;
use co_core_co::Co;
use co_primitives::{BlockStorage, BlockStorageExt, StoreParams};
use co_runtime::{RuntimeContext, RuntimePool};

/// Reference count state in a [`co_core_storage::Storage`] core.
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
	C: CoreResolver<S> + Send + Sync + 'static,
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
		let next = self.next.execute(storage, runtime, context, state, action).await?;

		// references
		if let Some(next_state) = next.state {
			let co_state: Co = storage.get_deserialized(&next_state).await?;
			if let Some(storage_state) = co_state.cores.get(CO_CORE_NAME_STORAGE) {
				let dispatch = RuntimeDispatch::new(
					runtime.clone(),
					storage.clone(),
					CO_CORE_NAME_STORAGE.to_owned(),
					Cores::default().core(CO_CORE_STORAGE).expect("co storage binary"),
					storage_state.state,
				);
				let reference_writer =
					ReferenceWriter::new(dispatch, self.reducer_context.clone(), self.pinning_key.clone());
				reference_writer
					.write(*state, next_state, <S::StoreParams as StoreParams>::MAX_BLOCK_SIZE)
					.await?;
			}
		}

		// result
		Ok(next)
	}
}
