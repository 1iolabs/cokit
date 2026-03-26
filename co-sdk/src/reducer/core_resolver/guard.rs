// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{CoreResolver, CoreResolverContext, CoreResolverError, Guards, RuntimeHandle};
use async_trait::async_trait;
use cid::Cid;
use co_guard::{GuardDefinition, GuardError, GuardResolver};
use co_runtime::RuntimeContext;
use co_storage::{BlockStorageExt, ExtendedBlockStorage};

#[derive(Debug, Clone)]
pub struct CoGuardResolver<C> {
	guard_resolver: GuardResolver,
	next: C,
}
impl<C> CoGuardResolver<C> {
	pub fn new(core_resolver: C, guards: &Guards) -> Self {
		Self { next: core_resolver, guard_resolver: GuardResolver::new(guards.mapping()) }
	}

	pub fn with_ignore_mode(mut self, ignore: bool) -> Self {
		self.guard_resolver = self.guard_resolver.with_ignore_mode(ignore);
		self
	}

	pub fn with_failure_mode(mut self) -> Self {
		self.guard_resolver = self.guard_resolver.with_failure_mode();
		self
	}
}
#[async_trait]
impl<S, C> CoreResolver<S> for CoGuardResolver<C>
where
	S: ExtendedBlockStorage + Send + Sync + Clone + 'static,
	C: CoreResolver<S> + Send + Sync + 'static,
{
	async fn execute(
		&self,
		storage: &S,
		runtime: &RuntimeHandle,
		context: &CoreResolverContext,
		state: &Option<Cid>,
		action: &Cid,
	) -> Result<RuntimeContext, CoreResolverError> {
		// verify guards
		if let Some(state) = *state {
			let co_state: co_core_co::Co = storage.get_deserialized(&state).await?;
			let guard_defs: std::collections::BTreeMap<String, GuardDefinition> = co_state
				.guards
				.into_iter()
				.map(|(name, guard)| (name, GuardDefinition { binary: guard.binary, tags: guard.tags }))
				.collect();
			let heads = context.entry.entry().next.clone();
			let next_head = *context.entry.cid();

			match self
				.guard_resolver
				.verify_guards(runtime, storage, &guard_defs, &state, &heads, &next_head)
				.await
			{
				Ok(()) => {},
				Err(GuardError::Skipped(_message, result)) => return Ok(result),
				Err(GuardError::Rejected(message)) => {
					return Err(CoreResolverError::Middleware(anyhow::anyhow!(message)))
				},
				Err(GuardError::Execute(err)) => return Err(CoreResolverError::Middleware(err)),
			}
		}

		// next
		let result = self.next.execute(storage, runtime, context, state, action).await?;

		// result
		Ok(result)
	}
}
