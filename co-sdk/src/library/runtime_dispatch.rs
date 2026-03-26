// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use super::create_reducer_action::create_reducer_action;
use crate::{services::runtime::RuntimeHandle, types::co_dispatch::CoDispatch, CoreResolverError};
use async_trait::async_trait;
use cid::Cid;
use co_identity::PrivateIdentityBox;
use co_primitives::{ReducerInput, StaticCoDate};
use co_runtime::{Core, RuntimeContext};
use co_storage::ExtendedBlockStorage;
use serde::Serialize;
use std::{fmt::Debug, marker::PhantomData};

/// Dispatch for implicit core resolver actions.
pub struct RuntimeDispatch<S, A> {
	identity: PrivateIdentityBox,
	runtime: RuntimeHandle,
	storage: S,
	core_name: String,
	core_binary: Cid,
	core: Core,
	state: Option<Cid>,
	_action: PhantomData<A>,
}

impl<S, A> RuntimeDispatch<S, A> {
	pub fn new(
		identity: PrivateIdentityBox,
		runtime: RuntimeHandle,
		storage: S,
		core_name: String,
		core_binary: Cid,
		core: Core,
		state: Option<Cid>,
	) -> Self {
		Self { identity, runtime, storage, core_name, core_binary, core, state, _action: PhantomData }
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
		let runtime_context = self
			.runtime
			.execute_state(
				&self.storage,
				&self.core_binary,
				&self.core,
				RuntimeContext::new(&ReducerInput { state: self.state, action: action_cid.into() })?,
			)
			.await
			.map_err(|e| CoreResolverError::Execute(self.core_name.clone(), e))?;

		// remove action
		// TODO: put this action into an "overlay storage" which used only memory?
		// TODO: make sure it not in use by anyone else?
		self.storage.remove(action_cid.cid()).await?;

		// check diagnostics
		//  (albeit they only happen due to bugs)
		//  we should always use diagnostics for implicit actions to not silently fail tasks
		runtime_context.ok()?;

		// result
		Ok(runtime_context.state)
	}
}
