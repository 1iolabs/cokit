use crate::{
	library::runtime_dispatch::RuntimeDispatch, types::co_dispatch::CoDispatch, CoreResolver, CoreResolverError, Cores,
	ReducerChangeContext, CO_CORE_CO, CO_CORE_NAME_CO,
};
use anyhow::Context;
use async_trait::async_trait;
use cid::Cid;
use co_core_co::CoAction;
use co_identity::{LocalIdentity, PrivateIdentity};
use co_primitives::ReducerAction;
use co_runtime::{Core, RuntimeContext, RuntimePool};
use co_storage::{BlockStorage, BlockStorageExt, ExtendedBlockStorage};
use ipld_core::ipld::Ipld;
use serde::de::IgnoredAny;
use std::collections::HashMap;

/// Resolve action core assuming the Co root state is to [`co_core_co::Co`].
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

	async fn apply_core_state_to_root<S: BlockStorage + 'static>(
		&self,
		storage: &S,
		runtime: RuntimePool,
		state: &Option<Cid>,
		core_name: String,
		core_state: Option<Cid>,
	) -> Result<Option<Cid>, anyhow::Error>
	where
		S: ExtendedBlockStorage + Send + Sync + Clone + 'static,
	{
		let mut dispatch = RuntimeDispatch::new(
			LocalIdentity::device().boxed(),
			runtime.clone(),
			storage.clone(),
			CO_CORE_NAME_CO.to_owned(),
			self.root_core(),
			*state, // we assume the root state points to [`co_core_co::Co`].
		);
		dispatch
			.dispatch(&co_core_co::CoAction::CoreChange { core: core_name.to_owned(), state: core_state })
			.await
			.context("apply to root")
	}

	async fn core_state_binary<S: BlockStorage + 'static>(
		&self,
		storage: &S,
		state: &Option<Cid>,
		core: &str,
	) -> Result<(bool, Option<Cid>, Core), CoreResolverError>
	where
		S: ExtendedBlockStorage + Send + Sync + Clone + 'static,
	{
		let root = core == CO_CORE_NAME_CO;
		if root {
			Ok((root, *state, self.root_core()))
		} else {
			// get root state
			let state: co_core_co::Co = storage.get_default(state).await?;

			// get core
			let core: &co_core_co::Core = state
				.cores
				.get(core)
				.ok_or_else(|| CoreResolverError::CoreNotFound(core.to_owned()))?;

			// resolve from known
			Ok((root, core.state, self.core(core.binary)))
		}
	}

	async fn migrate<S>(
		&self,
		storage: &S,
		runtime: &RuntimePool,
		state: &Option<Cid>,
		core_name: &str,
		migrate: &Cid,
	) -> Result<Option<Cid>, anyhow::Error>
	where
		S: ExtendedBlockStorage + Send + Sync + Clone + 'static,
	{
		// get core
		let (root, core_state, core) = self.core_state_binary(storage, state, core_name).await?;
		assert!(root == false);

		// read migrate
		let migrate: Ipld = storage.get_deserialized(migrate).await?;

		// apply migrate
		let mut core_dispatch = RuntimeDispatch::<S, Ipld>::new(
			LocalIdentity::device().boxed(),
			runtime.clone(),
			storage.clone(),
			core_name.to_owned(),
			core,
			core_state,
		);
		let core_state_migrate = core_dispatch.dispatch(&migrate).await?;

		// apply to root
		let result = self
			.apply_core_state_to_root(storage, runtime.clone(), state, core_name.to_owned(), core_state_migrate)
			.await?;

		// result
		Ok(result)
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
	S: ExtendedBlockStorage + Send + Sync + Clone + 'static,
{
	async fn execute(
		&self,
		storage: &S,
		runtime: &RuntimePool,
		_context: &ReducerChangeContext,
		state: &Option<Cid>,
		action: &Cid,
	) -> Result<RuntimeContext, CoreResolverError> {
		// get action
		let reducer_action: ReducerAction<IgnoredAny> = storage
			.get_deserialized(action)
			.await
			.map_err(|e| CoreResolverError::InvalidArgument(e.into()))
			.context("resolving action")?;

		// find core
		let (root, core_state, core) = self.core_state_binary(storage, state, &reducer_action.core).await?;

		// apply to state
		let mut result = runtime
			.execute(storage, &core, RuntimeContext::new(core_state, action.into()))
			.await
			.map_err(|e| CoreResolverError::Execute(reducer_action.core.clone(), e))?;

		// log
		#[cfg(feature = "logging-verbose")]
		{
			let previous_ipld = match core_state {
				Some(core_state) if co_primitives::MultiCodec::is_cbor(core_state) => {
					crate::ipld_resolve_recursive(storage, ipld_core::ipld::Ipld::Link(core_state), true).await?
				},
				_ => ipld_core::ipld::Ipld::Null,
			};
			let action_ipld = if co_primitives::MultiCodec::is_cbor(action) {
				crate::ipld_resolve_recursive(storage, ipld_core::ipld::Ipld::Link(*action), true).await?
			} else {
				ipld_core::ipld::Ipld::Null
			};
			let next_ipld = match result.state {
				Some(core_state) if co_primitives::MultiCodec::is_cbor(core_state) => {
					crate::ipld_resolve_recursive(storage, ipld_core::ipld::Ipld::Link(core_state), true).await?
				},
				_ => ipld_core::ipld::Ipld::Null,
			};
			tracing::trace!(
				core = reducer_action.core,
				previous_cid = ?core_state,
				?previous_ipld,
				action_cid = ?action,
				?action_ipld,
				next_cid = ?result.state,
				?next_ipld,
				"core-execute"
			);
		}

		// apply to root
		if !root {
			result.state = self
				.apply_core_state_to_root(storage, runtime.clone(), state, reducer_action.core, result.state)
				.await?;
		}

		// migrate?
		if root {
			let co_action: ReducerAction<CoAction> = storage
				.get_deserialized(action)
				.await
				.map_err(|e| CoreResolverError::InvalidArgument(e.into()))
				.context("resolving CoAction")?;
			match &co_action.payload {
				CoAction::CoreUpgrade { core, binary: _, migrate: Some(migrate) } => {
					result.state = self.migrate(storage, runtime, &result.state, core, migrate).await?;
				},
				_ => {},
			}
		}

		// result
		Ok(result)
	}
}
