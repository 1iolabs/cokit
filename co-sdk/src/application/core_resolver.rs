use crate::{Cores, CO_CORE_CO};
use async_trait::async_trait;
use co_core_co::Co;
use co_primitives::ReducerAction;
use co_runtime::Core;
use co_storage::{BlockStorage, BlockStorageExt, StorageError};
use libipld::Cid;
use serde::de::IgnoredAny;
use std::collections::HashMap;

#[async_trait]
pub trait CoreResolver {
	/// Resolve the COre responsible for reducing the action.
	async fn resolve_core(&self, action: &Cid) -> Result<Core, CoreResolverError>;

	/// Called when a rediced state has changed.
	async fn on_state_changed(&mut self, _co: &str, _state: Cid) -> Result<(), CoreResolverError> {
		Ok(())
	}
}

#[derive(Debug, thiserror::Error)]
pub enum CoreResolverError {
	/// Storage error.
	#[error("Storage error")]
	Storage(#[from] StorageError),

	/// Invalid arguemnt (action) supplied to the resolver.
	#[error("Invalid argument")]
	InvalidArgument(#[from] anyhow::Error),

	/// The core referenced by the action can not be found.
	#[error("Core not found: {0}")]
	CoreNotFound(String),
}

#[derive(Debug, Clone)]
pub struct SingleCoreResolver {
	core: Core,
}
impl SingleCoreResolver {
	pub fn new(core: Core) -> Self {
		Self { core }
	}
}
#[async_trait]
impl CoreResolver for SingleCoreResolver {
	async fn resolve_core(&self, _action: &Cid) -> Result<Core, CoreResolverError> {
		Ok(self.core.clone())
	}
}

/// Resolve to core to use from
#[derive(Debug, Clone)]
pub struct CoCoreResolver<S: Clone> {
	storage: S,
	by_co_name: HashMap<String, Core>,
	state: Option<Co>,
}
impl<S> CoCoreResolver<S>
where
	S: BlockStorage + Send + Sync + Clone + 'static,
{
	pub fn new(storage: S, state: Option<Co>, co_core: Option<Core>) -> Self {
		let mut by_co_name = HashMap::<String, Core>::new();
		by_co_name.insert(Cores::to_core_name(CO_CORE_CO).to_owned(), co_core.unwrap_or(Core::native::<Co>()));
		Self { storage, state, by_co_name }
	}

	pub fn insert_core(&mut self, co: String, core: Core) {
		self.by_co_name.insert(co, core);
	}

	pub fn set_state(&mut self, state: Option<Co>) {
		self.state = state;
	}
}
#[async_trait]
impl<S> CoreResolver for CoCoreResolver<S>
where
	S: BlockStorage + Send + Sync + Clone + 'static,
{
	async fn resolve_core(&self, action: &Cid) -> Result<Core, CoreResolverError> {
		// get action
		let reducer_action: ReducerAction<IgnoredAny> = self
			.storage
			.get_deserialized(action)
			.await
			.map_err(|e| CoreResolverError::InvalidArgument(e.into()))?;

		// resolve from known
		if let Some(core) = self.by_co_name.get(&reducer_action.core) {
			return Ok(core.clone());
		}

		// resolve from co state
		self.state
			.as_ref()
			.ok_or_else(|| CoreResolverError::CoreNotFound(reducer_action.core.to_owned()))?
			.cores
			.get(&reducer_action.core)
			.ok_or_else(|| CoreResolverError::CoreNotFound(reducer_action.core))
			.map(|core| Core::Wasm(core.binary))
	}

	async fn on_state_changed(&mut self, co: &str, state: Cid) -> Result<(), CoreResolverError> {
		if co == Cores::to_core_name(CO_CORE_CO) {
			self.set_state(self.storage.get_deserialized(&state).await?);
		}
		Ok(())
	}
}

/// Mapping core resolve.
/// Can be used to map WebAssembly core Cids to Native versions.
#[derive(Debug)]
pub struct MappingCoreResolver<R> {
	next: R,
	mapping: HashMap<Cid, Core>,
}
impl<R> MappingCoreResolver<R> {
	pub fn new(next: R) -> Self {
		Self { next, mapping: Default::default() }
	}

	pub fn insert(&mut self, from: Cid, to: Core) {
		self.mapping.insert(from, to);
	}
}
#[async_trait]
impl<R> CoreResolver for MappingCoreResolver<R>
where
	R: CoreResolver + Sync + Send,
{
	async fn resolve_core(&self, action: &Cid) -> Result<Core, CoreResolverError> {
		self.next.resolve_core(action).await.map(|core| match core {
			Core::Wasm(cid) => self.mapping.get(&cid).cloned().unwrap_or(Core::Wasm(cid)),
			core => core,
		})
	}
}
