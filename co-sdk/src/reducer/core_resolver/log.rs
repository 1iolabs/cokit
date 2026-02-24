// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{services::runtime::RuntimeHandle, CoreResolver, CoreResolverContext, CoreResolverError};
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{BlockStorage, BlockStorageExt, CoDate, CoId, DiagnosticMessage, DynamicCoDate, ReducerAction};
use co_runtime::RuntimeContext;
use ipld_core::ipld::Ipld;

#[derive(Debug, Clone)]
pub struct LogCoreResolver<C> {
	next: C,
	co: CoId,
	date: DynamicCoDate,
}
impl<C> LogCoreResolver<C> {
	pub fn new(core_resolver: C, co: CoId, date: DynamicCoDate) -> Self {
		Self { next: core_resolver, co, date }
	}
}
#[async_trait]
impl<S, C> CoreResolver<S> for LogCoreResolver<C>
where
	S: BlockStorage + Send + Sync + Clone + 'static,
	C: CoreResolver<S> + Send + Sync + 'static,
{
	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), skip_all)]
	async fn execute(
		&self,
		storage: &S,
		runtime: &RuntimeHandle,
		context: &CoreResolverContext,
		state: &Option<Cid>,
		action: &Cid,
	) -> Result<RuntimeContext, CoreResolverError> {
		let start = self.date.now();
		let mut runtime_context = self.next.execute(storage, runtime, context, state, action).await?;

		// log
		let action_ipld: Option<ReducerAction<Ipld>> = storage.get_deserialized(action).await.ok();
		let duration = self.date.now() - start;
		tracing::trace!(
			co = ?self.co,
			previous_state = ?state,
			next_state = ?runtime_context.state,
			head = ?context.entry.cid(),
			?duration,
			?action_ipld,
			?action,
			"reducer-action"
		);

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
