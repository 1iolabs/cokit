use super::identity::create_identity_resolver;
use crate::{
	reducer::core_resolver::dynamic::DynamicCoreResolver, CoCoreResolver, CoDate, CoReducerState, Reducer,
	ReducerBuilder,
};
use co_log::Log;
use co_primitives::CoId;
use co_runtime::RuntimePool;
use co_storage::ExtendedBlockStorage;

/// Create a memory instance.
pub async fn create_memory_reducer<S>(
	runtime_pool: &RuntimePool,
	date: impl CoDate,
	id: &CoId,
	storage: &S,
	reducer_state: CoReducerState,
) -> Result<Reducer<S, DynamicCoreResolver<S>>, anyhow::Error>
where
	S: ExtendedBlockStorage + Clone + 'static,
{
	let log = Log::new(id.as_bytes().to_vec(), create_identity_resolver(), Default::default());
	let core_resolver = CoCoreResolver::default();
	let mut builder = ReducerBuilder::new(DynamicCoreResolver::new(core_resolver), log);
	if let Some((state, heads)) = reducer_state.some() {
		builder = builder.with_latest_state(state, heads);
	}
	let reducer = builder.build(storage, runtime_pool, date).await?;
	Ok(reducer)
}
