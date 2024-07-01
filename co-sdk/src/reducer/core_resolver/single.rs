use crate::{CoreResolver, CoreResolverError};
use async_trait::async_trait;
use co_runtime::{Core, RuntimeContext, RuntimePool};
use co_storage::BlockStorage;
use libipld::Cid;

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
		state: &Option<Cid>,
		action: &Cid,
	) -> Result<Option<Cid>, CoreResolverError> {
		Ok(runtime
			.execute(storage, &self.core, RuntimeContext { state: *state, event: action.into() })
			.await
			.map_err(|e| CoreResolverError::Execute("root".to_owned(), e))?)
	}
}
