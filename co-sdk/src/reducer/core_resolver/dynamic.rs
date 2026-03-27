// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{CoreResolver, CoreResolverContext, CoreResolverError};
use async_trait::async_trait;
use cid::Cid;
use co_runtime::{RuntimeContext, RuntimeHandle};
use co_storage::BlockStorage;
use std::{
	fmt::{Debug, Formatter},
	sync::Arc,
};

#[derive(Clone)]
pub struct DynamicCoreResolver<S> {
	inner: Arc<dyn CoreResolver<S> + Send + Sync + 'static>,
}

impl<S> Debug for DynamicCoreResolver<S> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("DynamicCoreResolver").finish()
	}
}
impl<S> DynamicCoreResolver<S>
where
	S: BlockStorage + Send + Sync + Clone + 'static,
{
	pub fn new<R>(core_resolver: R) -> Self
	where
		R: CoreResolver<S> + Send + Sync + 'static,
	{
		Self { inner: Arc::new(core_resolver) }
	}
}
#[async_trait]
impl<S> CoreResolver<S> for DynamicCoreResolver<S>
where
	S: BlockStorage + Clone + Send + Sync + 'static,
{
	async fn execute(
		&self,
		storage: &S,
		runtime: &RuntimeHandle,
		context: &CoreResolverContext,
		state: &Option<Cid>,
		action: &Cid,
	) -> Result<RuntimeContext, CoreResolverError> {
		self.inner.execute(storage, runtime, context, state, action).await
	}
}
