use crate::{CoreResolver, CoreResolverError, Cores, ReducerChangeContext, CO_CORE_CO, CO_CORE_NAME_CO};
use anyhow::Context;
use async_trait::async_trait;
use co_primitives::ReducerAction;
use co_runtime::{Core, RuntimeContext, RuntimePool};
use co_storage::{BlockStorage, BlockStorageExt};
use cid::Cid;
use serde::de::IgnoredAny;
use std::collections::HashMap;

/// Resolve action core assuming the Co root state is to `co_core_co::Co`.
#[derive(Debug, Clone)]
pub struct CoCoreResolver {
	mapping: HashMap<Cid, Core>,
}
impl CoCoreResolver {
	pub fn with_mapping(mapping: HashMap<Cid, Core>) -> Self {
		Self { mapping }
	}

	fn core(&self, wasm: Cid) -> Core {
		self.mapping.get(&wasm).cloned().unwrap_or(Core::Wasm(wasm))
	}

	fn root_core(&self) -> Core {
		self.core(Cores::default().binary(CO_CORE_CO).expect("co core binary"))
	}
}
impl Default for CoCoreResolver {
	fn default() -> Self {
		Self::with_mapping(Cores::default().built_in_native_mapping())
	}
}
#[async_trait]
impl<S> CoreResolver<S> for CoCoreResolver
where
	S: BlockStorage + Send + Sync + Clone + 'static,
{
	async fn execute(
		&self,
		storage: &S,
		runtime: &RuntimePool,
		_context: &ReducerChangeContext,
		state: &Option<Cid>,
		action: &Cid,
	) -> Result<Option<Cid>, CoreResolverError> {
		// get action
		let reducer_action: ReducerAction<IgnoredAny> = storage
			.get_deserialized(action)
			.await
			.map_err(|e| CoreResolverError::InvalidArgument(e.into()))
			.context("resolving action")?;

		// find core
		let root = reducer_action.core == CO_CORE_NAME_CO;
		let (core_state, core) = if root {
			(*state, self.root_core())
		} else {
			// get root state
			let state: co_core_co::Co = storage.get_default(state).await?;

			// get core
			let core: &co_core_co::Core = state
				.cores
				.get(&reducer_action.core)
				.ok_or_else(|| CoreResolverError::CoreNotFound(reducer_action.core.clone()))?;

			// resolve from known
			(core.state, self.core(core.binary))
		};

		// apply to state
		let mut result = runtime
			.execute(storage, &core, RuntimeContext { state: core_state, event: action.into() })
			.await
			.map_err(|e| CoreResolverError::Execute(reducer_action.core.clone(), e))?;

		// apply to root
		if !root {
			// Note: this action must be deterministic so we pass no time otherwise when we retry this could introduce
			// random values.
			let action: ReducerAction<co_core_co::CoAction> = ReducerAction {
				core: CO_CORE_NAME_CO.to_owned(),
				from: "did:local:device".to_owned(),
				payload: co_core_co::CoAction::CoreChange { core: reducer_action.core.clone(), state: result },
				time: 0,
			};
			let action_cid = storage.set_serialized(&action).await?;

			// apply
			result = runtime
				.execute(storage, &self.root_core(), RuntimeContext { state: *state, event: action_cid })
				.await
				.map_err(|e| CoreResolverError::Execute(reducer_action.core.clone(), e))?;

			// remove action
			// TODO: put this action into an "overlay storage" which used only memory?
			storage.remove(&action_cid).await?;
		}

		// result
		Ok(result)
	}
}
