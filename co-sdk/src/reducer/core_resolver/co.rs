use crate::{
	library::runtime_dispatch::RuntimeDispatch, types::co_dispatch::CoDispatch, CoreResolver, CoreResolverContext,
	CoreResolverError, Cores, CO_CORE_NAME_CO,
};
use anyhow::Context;
use async_trait::async_trait;
use cid::Cid;
use co_core_co::{CoAction, CreateAction};
use co_identity::{LocalIdentity, PrivateIdentity};
use co_primitives::ReducerAction;
use co_runtime::{Core, RuntimeContext, RuntimePool};
use co_storage::{BlockStorage, BlockStorageExt, ExtendedBlockStorage};
use ipld_core::ipld::Ipld;
use serde::Deserialize;
use std::collections::HashMap;

/// Resolve action core assuming the Co root state is to [`co_core_co::Co`].
#[derive(Debug, Clone)]
pub struct CoCoreResolver {
	mapping: HashMap<Cid, Core>,
}
impl CoCoreResolver {
	pub fn new(cores: &Cores) -> Self {
		Self::with_mapping(cores.mapping())
	}

	pub fn with_mapping(mapping: HashMap<Cid, Core>) -> Self {
		Self { mapping }
	}

	fn core(&self, wasm: Cid) -> Core {
		self.mapping.get(&wasm).cloned().unwrap_or(Core::Wasm(wasm))
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
		let (_root, _name, state, core_binary, core) = self
			.core_state_binary(storage, state, CoreSource::Name(CO_CORE_NAME_CO.as_ref()))
			.await?;
		let mut dispatch = RuntimeDispatch::new(
			LocalIdentity::device().boxed(),
			runtime.clone(),
			storage.clone(),
			CO_CORE_NAME_CO.to_string(),
			core_binary,
			core,
			state,
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
		core: CoreSource<'_>,
	) -> Result<(bool, String, Option<Cid>, Cid, Core), CoreResolverError>
	where
		S: ExtendedBlockStorage + Send + Sync + Clone + 'static,
	{
		// get core source
		let core_name = match core {
			CoreSource::Name(name) => name.to_owned(),
			CoreSource::Action(action) => {
				let reducer_action: CoreReducerAction = storage
					.get_deserialized(&action)
					.await
					.map_err(|e| CoreResolverError::InvalidArgument(e.into()))
					.context("resolving CoreReducerAction")?;
				reducer_action.core
			},
		};

		let root = CO_CORE_NAME_CO == core_name;
		let (core_name, core_state, core_binary) = if root {
			let co_binary = if state.is_none() {
				// in case we have no co state we expect the action is a create
				// we need to use the binary from the create call because the creator is allowed to create a co with an
				// older/newer core than the buildin
				let action = match core {
					CoreSource::Name(_) => {
						return Err(CoreResolverError::InvalidArgument(anyhow::anyhow!("No co core binary")))
					},
					CoreSource::Action(cid) => cid,
				};
				let co_action: ReducerAction<CoAction> = storage
					.get_deserialized(&action)
					.await
					.map_err(|e| CoreResolverError::InvalidArgument(e.into()))
					.context("resolving CoAction::Create")?;
				match co_action.payload {
					CoAction::Create(CreateAction { binary, .. }) => binary,
					_ => {
						return Err(CoreResolverError::InvalidArgument(anyhow::anyhow!(
							"Execute before CoAction::Create: {:?}",
							co_action.payload,
						)))
					},
				}
			} else {
				// get co core binary from state
				let co_state: co_core_co::Co = storage.get_default(state).await?;
				co_state.binary
			};
			(core_name, *state, co_binary)
		} else {
			// get core binary from state
			let co_state: co_core_co::Co = storage.get_default(state).await?;
			let core: &co_core_co::Core = co_state
				.cores
				.get(&core_name)
				.ok_or_else(|| CoreResolverError::CoreNotFound(core_name.clone()))?;
			(core_name, core.state, core.binary)
		};

		// result
		Ok((root, core_name, core_state, core_binary, self.core(core_binary)))
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
		let (root, _, core_state, core_binary, core) =
			self.core_state_binary(storage, state, CoreSource::Name(core_name)).await?;

		// read migrate
		let migrate: Ipld = storage.get_deserialized(migrate).await?;

		// apply migrate
		let mut core_dispatch = RuntimeDispatch::<S, Ipld>::new(
			LocalIdentity::device().boxed(),
			runtime.clone(),
			storage.clone(),
			core_name.to_owned(),
			core_binary,
			core,
			core_state,
		);
		let mut result = core_dispatch.dispatch(&migrate).await?;

		// apply to root
		if !root {
			result = self
				.apply_core_state_to_root(storage, runtime.clone(), state, core_name.to_owned(), result)
				.await?;
		}

		// result
		Ok(result)
	}
}
impl Default for CoCoreResolver {
	fn default() -> Self {
		Self::new(&Cores::default())
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
		context: &CoreResolverContext,
		state: &Option<Cid>,
		action: &Cid,
	) -> Result<RuntimeContext, CoreResolverError> {
		// find core
		let (root, core_name, core_state, core_binary, core) =
			self.core_state_binary(storage, state, CoreSource::Action(*action)).await?;

		// apply to state
		//  use precomputed state if specified
		let mut result = if let Some(result_core_state) = context.state {
			RuntimeContext::new(Some(result_core_state), action.into())
		} else {
			runtime
				.execute_state(storage, &core_binary, &core, RuntimeContext::new(core_state, action.into()))
				.await
				.map_err(|e| CoreResolverError::Execute(core_name.clone(), e))?
		};

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
				core = core_name,
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
				.apply_core_state_to_root(storage, runtime.clone(), state, core_name, result.state)
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
				CoAction::Upgrade { binary: _, migrate: Some(migrate) } => {
					result.state = self
						.migrate(storage, runtime, &result.state, CO_CORE_NAME_CO.as_ref(), migrate)
						.await?;
				},
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

enum CoreSource<'a> {
	Name(&'a str),
	Action(Cid),
}

/// Only extracts the core of an reducer action.
/// See: [`co_primitives::ReducerAction`]
#[derive(Debug, Deserialize)]
struct CoreReducerAction {
	#[serde(rename = "c")]
	core: String,
}

#[cfg(test)]
mod tests {
	use crate::reducer::core_resolver::co::CoreReducerAction;
	use co_core_co::CoAction;
	use co_primitives::{from_cbor, tags, to_cbor, ReducerAction};

	#[test]
	fn test_core_reducer_action() {
		let reducer_action = ReducerAction {
			core: "test-core".into(),
			from: "did:test".into(),
			payload: CoAction::TagsInsert { tags: tags!("hello": "world") },
			time: 1,
		};
		let reducer_action_cbor = to_cbor(&reducer_action).unwrap();
		let core_reducer_action: CoreReducerAction = from_cbor(&reducer_action_cbor).unwrap();
		assert_eq!(core_reducer_action.core.as_str(), "test-core");
	}
}
