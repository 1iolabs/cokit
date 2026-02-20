use super::identity::create_identity_resolver;
use crate::{
	reducer::core_resolver::dynamic::DynamicCoreResolver, services::runtime::RuntimeHandle, CoCoreResolver, CoDate,
	CoReducerState, Reducer, ReducerBuilder,
};
use co_log::{IdentityEntryVerifier, Log};
use co_primitives::CoId;
use co_storage::ExtendedBlockStorage;

/// Create a memory instance.
pub async fn create_memory_reducer<S>(
	runtime: &RuntimeHandle,
	date: impl CoDate,
	id: &CoId,
	storage: &S,
	core_resolver: Option<DynamicCoreResolver<S>>,
	reducer_state: CoReducerState,
) -> Result<Reducer<S, DynamicCoreResolver<S>>, anyhow::Error>
where
	S: ExtendedBlockStorage + Clone + 'static,
{
	let log =
		Log::new(id.as_bytes().to_vec(), IdentityEntryVerifier::new(create_identity_resolver()), reducer_state.heads());
	let core_resolver = core_resolver.unwrap_or_else(|| {
		let core_resolver = CoCoreResolver::default();

		DynamicCoreResolver::new(core_resolver)
	});
	let mut builder = ReducerBuilder::new(core_resolver, log);
	if let Some((state, heads)) = reducer_state.some() {
		builder = builder.with_latest_state(state, heads);
	}
	let reducer = builder.build(storage, runtime, date).await?;
	Ok(reducer)
}
