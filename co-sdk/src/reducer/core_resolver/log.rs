use crate::{CoreResolver, CoreResolverError, ReducerChangeContext};
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{BlockStorage, BlockStorageExt, DiagnosticMessage};
use co_runtime::{RuntimeContext, RuntimePool};

#[derive(Debug, Clone)]
pub struct LogCoreResolver<C> {
	next: C,
}
impl<C> LogCoreResolver<C> {
	pub fn new(core_resolver: C) -> Self {
		Self { next: core_resolver }
	}
}
#[async_trait]
impl<S, C> CoreResolver<S> for LogCoreResolver<C>
where
	S: BlockStorage + Send + Sync + Clone + 'static,
	C: CoreResolver<S> + Send + Sync + 'static,
{
	#[tracing::instrument(level = tracing::Level::TRACE, err, ret, skip(self, storage, runtime))]
	async fn execute(
		&self,
		storage: &S,
		runtime: &RuntimePool,
		context: &ReducerChangeContext,
		state: &Option<Cid>,
		action: &Cid,
	) -> Result<RuntimeContext, CoreResolverError> {
		let runtime_context = self.next.execute(storage, runtime, context, state, action).await?;

		// trace diagnostics
		for diagnostic_cid in runtime_context.diagnostics.iter() {
			if let Some(diagnostic) = storage.get_deserialized::<DiagnosticMessage>(diagnostic_cid).await.ok() {
				match diagnostic {
					DiagnosticMessage::Failure(err) => {
						tracing::error!(err, "action-failed");
					},
				}
			}
		}

		// result
		Ok(runtime_context)
	}
}
