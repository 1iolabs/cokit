use crate::{types::co_dispatch::CoDispatch, CoreResolver, ReducerChangeContext};
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{BlockStorage, BlockStorageExt, ReducerAction};
use co_runtime::RuntimePool;
use serde::Serialize;
use std::{fmt::Debug, marker::PhantomData};

/// Dispatch for implicit core resolver actions.
pub struct CoreResolverDispatch<C, S, A> {
	core_resolver: C,
	runtime: RuntimePool,
	context: ReducerChangeContext,
	storage: S,
	core_name: String,
	state: Option<Cid>,
	_action: PhantomData<A>,
}
impl<'c, C, S, A> CoreResolverDispatch<C, S, A> {
	pub fn new(
		core_resolver: C,
		runtime: RuntimePool,
		context: ReducerChangeContext,
		storage: S,
		core_name: String,
		state: Option<Cid>,
	) -> Self {
		Self { core_resolver, runtime, context, storage, core_name, state, _action: PhantomData }
	}
}
#[async_trait]
impl<C, S, A> CoDispatch<A> for CoreResolverDispatch<C, S, A>
where
	C: CoreResolver<S> + Send + Sync + 'static,
	S: BlockStorage + Send + Sync + Clone + 'static,
	A: Serialize + Debug + Send + Sync + Clone + 'static,
{
	async fn dispatch(&mut self, action: &A) -> Result<Option<Cid>, anyhow::Error> {
		// Note: this action must be deterministic so we pass no time otherwise when we retry this could introduce
		// random values.
		let reducer_action: ReducerAction<&A> = ReducerAction {
			core: self.core_name.clone(),
			from: "did:local:device".to_owned(),
			payload: action,
			time: 0,
		};
		let action_cid = self.storage.set_serialized(&reducer_action).await?;

		// apply
		let runtime_context = self
			.core_resolver
			.execute(&self.storage, &self.runtime, &self.context, &self.state, &action_cid)
			.await?;

		// log
		#[cfg(feature = "logging-verbose")]
		tracing::trace!(?action, previous_state = ?self.state, next_state = ?runtime_context.state, "core-dispatch");

		// remove action
		// TODO: put this action into an "overlay storage" which used only memory?
		// TODO: make sure it not in use by anyone else?
		self.storage.remove(&action_cid).await?;

		// propagate failures as this is meant for internal actions which should not fail
		// - this indicates a bug in sdk internals if it fails
		// - actions dispatched here will be created for the current state and not expected to fail.
		// - actions dispatched here are implicit (not in the heads) so they get recomputet every time the head is
		//   (re-)executed.
		runtime_context.ok(&self.storage).await?;

		// update
		self.state = runtime_context.state;

		// result
		Ok(runtime_context.state)
	}
}
