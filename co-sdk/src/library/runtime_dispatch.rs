use super::create_reducer_action::create_reducer_action;
use crate::{types::co_dispatch::CoDispatch, CoreResolverError, StaticCoDate};
use async_trait::async_trait;
use cid::Cid;
use co_identity::PrivateIdentityBox;
use co_runtime::{Core, RuntimeContext, RuntimePool};
use co_storage::ExtendedBlockStorage;
use serde::Serialize;
use std::{fmt::Debug, marker::PhantomData};

/// Dispatch for implicit core resolver actions.
pub struct RuntimeDispatch<S, A> {
	identity: PrivateIdentityBox,
	runtime: RuntimePool,
	storage: S,
	core_name: String,
	core: Core,
	state: Option<Cid>,
	_action: PhantomData<A>,
}

impl<S, A> RuntimeDispatch<S, A> {
	pub fn new(
		identity: PrivateIdentityBox,
		runtime: RuntimePool,
		storage: S,
		core_name: String,
		core: Core,
		state: Option<Cid>,
	) -> Self {
		Self { identity, runtime, storage, core_name, core, state, _action: PhantomData }
	}
}
#[async_trait]
impl<S, A> CoDispatch<A> for RuntimeDispatch<S, A>
where
	S: ExtendedBlockStorage + Send + Sync + Clone + 'static,
	A: Serialize + Debug + Send + Sync + Clone + 'static,
{
	async fn dispatch(&mut self, action: &A) -> Result<Option<Cid>, anyhow::Error> {
		// Note: this action must be deterministic so we pass no time otherwise when we retry this could introduce
		// random values.
		let action_cid = create_reducer_action(
			&self.storage,
			&self.identity,
			&self.core_name,
			action,
			Default::default(),
			&StaticCoDate(0),
		)
		.await?;

		// apply
		let result = self
			.runtime
			.execute(&self.storage, &self.core, RuntimeContext::new(self.state, action_cid.into()))
			.await
			.map_err(|e| CoreResolverError::Execute(self.core_name.clone(), e))?;

		// remove action
		// TODO: put this action into an "overlay storage" which used only memory?
		// TODO: make sure it not in use by anyone else?
		self.storage.remove(action_cid.cid()).await?;

		// result
		Ok(result.state)
	}
}
