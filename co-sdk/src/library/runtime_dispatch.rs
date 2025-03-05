use crate::{types::co_dispatch::CoDispatch, CoreResolverError};
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{BlockStorage, BlockStorageExt, ReducerAction};
use co_runtime::{RuntimeContext, RuntimePool};
use serde::Serialize;
use std::{fmt::Debug, marker::PhantomData};

/// Dispatch for implicit core resolver actions.
pub struct RuntimeDispatch<S, A> {
	runtime: RuntimePool,
	storage: S,
	core_name: String,
	core: co_runtime::Core,
	state: Option<Cid>,
	_action: PhantomData<A>,
}

impl<S, A> RuntimeDispatch<S, A> {
	pub fn new(
		runtime: RuntimePool,
		storage: S,
		core_name: String,
		core: co_runtime::Core,
		state: Option<Cid>,
	) -> Self {
		Self { runtime, storage, core_name, core, state, _action: PhantomData }
	}
}
#[async_trait]
impl<S, A> CoDispatch<A> for RuntimeDispatch<S, A>
where
	S: BlockStorage + Send + Sync + Clone + 'static,
	A: Serialize + Debug + Send + Sync + Clone + 'static,
{
	async fn dispatch(&self, action: &A) -> Result<Option<Cid>, anyhow::Error> {
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
		let result = self
			.runtime
			.execute(&self.storage, &self.core, RuntimeContext::new(self.state, action_cid))
			.await
			.map_err(|e| CoreResolverError::Execute(reducer_action.core.clone(), e))?;

		// remove action
		// TODO: put this action into an "overlay storage" which used only memory?
		// TODO: make sure it not in use by anyone else?
		self.storage.remove(&action_cid).await?;

		// result
		Ok(result.state)
	}
}
