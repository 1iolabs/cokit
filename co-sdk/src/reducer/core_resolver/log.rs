use crate::{CoreResolver, CoreResolverContext, CoreResolverError};
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{BlockStorage, DiagnosticMessage};
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
	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), ret, skip(self, storage, runtime))]
	async fn execute(
		&self,
		storage: &S,
		runtime: &RuntimePool,
		context: &CoreResolverContext,
		state: &Option<Cid>,
		action: &Cid,
	) -> Result<RuntimeContext, CoreResolverError> {
		let mut runtime_context = self.next.execute(storage, runtime, context, state, action).await?;

		// resolve diagnostics
		runtime_context.resolve_diagnostics(storage).await?;

		// trace diagnostics
		for diagnostic in runtime_context.diagnostics.iter() {
			if let Some(message) = diagnostic.message() {
				match message {
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
