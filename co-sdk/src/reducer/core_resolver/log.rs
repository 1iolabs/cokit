use crate::{CoreResolver, CoreResolverError, ReducerChangeContext};
use async_trait::async_trait;
use co_runtime::RuntimePool;
use co_storage::BlockStorage;
use libipld::Cid;

pub struct LogCoreResolver<C> {
	next: C,
}
impl<C> LogCoreResolver<C> {
	pub fn new<S>(core_resolver: C) -> Self
	where
		S: BlockStorage + Send + Sync + Clone + 'static,
		C: CoreResolver<S> + Send + Sync + 'static,
	{
		Self { next: core_resolver }
	}
}
#[async_trait]
impl<S, C> CoreResolver<S> for LogCoreResolver<C>
where
	S: BlockStorage + Send + Sync + Clone + 'static,
	C: CoreResolver<S> + Send + Sync + 'static,
{
	#[tracing::instrument(err, ret, skip(self, storage, runtime))]
	async fn execute(
		&self,
		storage: &S,
		runtime: &RuntimePool,
		context: &ReducerChangeContext,
		state: &Option<Cid>,
		action: &Cid,
	) -> Result<Option<Cid>, CoreResolverError> {
		self.next.execute(storage, runtime, context, state, action).await
	}
}
