// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{services::runtime::RuntimeHandle, CoreResolver, CoreResolverContext, CoreResolverError};
use async_trait::async_trait;
use cid::Cid;
use co_runtime::{Core, RuntimeContext};
use co_storage::BlockStorage;

#[derive(Debug, Clone)]
pub struct SingleCoreResolver {
	core_binary: Cid,
	core: Core,
}
impl SingleCoreResolver {
	pub fn new(core_binary: Cid, core: Core) -> Self {
		Self { core_binary, core }
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
		runtime: &RuntimeHandle,
		_context: &CoreResolverContext,
		state: &Option<Cid>,
		action: &Cid,
	) -> Result<RuntimeContext, CoreResolverError> {
		Ok(runtime
			.execute_state(storage, &self.core_binary, &self.core, RuntimeContext::new(*state, action.into()))
			.await
			.map_err(|e| CoreResolverError::Execute("root".to_owned(), e))?)
	}
}
