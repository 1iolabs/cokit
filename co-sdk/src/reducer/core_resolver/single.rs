use crate::{CoreResolver, CoreResolverContext, CoreResolverError};
use async_trait::async_trait;
use cid::Cid;
use co_runtime::{Core, RuntimeContext, RuntimePool};
use co_storage::BlockStorage;

#[derive(Debug, Clone)]
pub struct SingleCoreResolver {
	core: Core,
}
impl SingleCoreResolver {
	pub fn new(core: Core) -> Self {
		Self { core }
	}
}
#[async_trait]
impl<S> CoreResolver<S> for SingleCoreResolver
where
	S: BlockStorage + Send + Sync + Clone + 'static,
{
	async fn execute(
		&self,
		storage: &S,
		runtime: &RuntimePool,
		_context: &CoreResolverContext,
		state: &Option<Cid>,
		action: &Cid,
	) -> Result<RuntimeContext, CoreResolverError> {
		Ok(runtime
			.execute_state(storage, &self.core, RuntimeContext::new(*state, action.into()))
			.await
			.map_err(|e| CoreResolverError::Execute("root".to_owned(), e))?)
	}
}
