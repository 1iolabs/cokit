// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{CoreResolver, CoreResolverContext, CoreResolverError};
use async_trait::async_trait;
use cid::Cid;
use co_guard::{CoreGuard, GuardDefinition, GuardError, Guards};
use co_runtime::{RuntimeContext, RuntimeHandle};
use co_storage::{BlockStorageExt, ExtendedBlockStorage};

#[derive(Debug, Clone)]
pub struct CoGuardResolver<C> {
	guard: CoreGuard,
	next: C,
}
impl<C> CoGuardResolver<C> {
	pub fn new(core_resolver: C, guards: &Guards) -> Self {
		Self { next: core_resolver, guard: CoreGuard::new(guards.mapping()) }
	}

	pub fn with_ignore_mode(mut self, ignore: bool) -> Self {
		self.guard = self.guard.with_ignore_mode(ignore);
		self
	}

	pub fn with_failure_mode(mut self) -> Self {
		self.guard = self.guard.with_failure_mode();
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
				.guard
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
