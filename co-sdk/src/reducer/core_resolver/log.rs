// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{CoreResolver, CoreResolverContext, CoreResolverError};
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{BlockStorage, BlockStorageExt, CoDate, CoId, DynamicCoDate, ReducerAction};
use co_runtime::{RuntimeContext, RuntimeHandle};
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
		let runtime_context = self.next.execute(storage, runtime, context, state, action).await?;

		// log
		let action_ipld: Option<ReducerAction<Ipld>> = storage.get_deserialized(action).await.ok();
		let duration = self.date.now() - start;

		// trace
		if let Some(Err(err)) = &runtime_context.result {
			tracing::error!(
				err,
				co = ?self.co,
				previous_state = ?state,
				next_state = ?runtime_context.state,
				head = ?context.entry.cid(),
				?duration,
				?action_ipld,
				?action,
				"reducer-action"
			);
		} else {
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
		}

		// result
		Ok(runtime_context)
	}
}
