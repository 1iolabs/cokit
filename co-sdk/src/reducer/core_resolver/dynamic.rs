use crate::{CoreResolver, CoreResolverError, ReducerChangeContext};
use async_trait::async_trait;
use cid::Cid;
use co_runtime::RuntimePool;
use co_storage::BlockStorage;

pub struct DynamicCoreResolver<S> {
	inner: Box<dyn CoreResolver<S> + Send + Sync + 'static>,
}
impl<S> DynamicCoreResolver<S>
where
	S: BlockStorage + Send + Sync + Clone + 'static,
{
	pub fn new<R>(core_resolver: R) -> Self
	where
		R: CoreResolver<S> + Send + Sync + 'static,
	{
		Self { inner: Box::new(core_resolver) }
	}
}
#[async_trait]
impl<S> CoreResolver<S> for DynamicCoreResolver<S>
where
	S: BlockStorage + Send + Sync + Clone + 'static,
{
	async fn execute(
		&self,
		storage: &S,
		runtime: &RuntimePool,
		context: &ReducerChangeContext,
		state: &Option<Cid>,
		action: &Cid,
	) -> Result<Option<Cid>, CoreResolverError> {
		self.inner.execute(storage, runtime, context, state, action).await
	}
}
