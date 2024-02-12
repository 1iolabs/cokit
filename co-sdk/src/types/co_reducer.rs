use crate::{CoCoreResolver, CoStorage, Reducer, Runtime, CO_CORE_NAME_CO};
use co_log::PrivateIdentity;
use co_storage::{BlockStorageExt, StorageError};
use libipld::Cid;
use serde::{de::DeserializeOwned, Serialize};
use std::{collections::BTreeSet, sync::Arc};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct CoReducer {
	pub(crate) reducer: Arc<RwLock<Reducer<CoStorage, CoCoreResolver>>>,
	pub(crate) storage: CoStorage,
	pub(crate) runtime: Runtime,
}
impl CoReducer {
	pub(crate) fn new(runtime: Runtime, reducer: Reducer<CoStorage, CoCoreResolver>) -> Self {
		Self { runtime, storage: reducer.log().storage().clone(), reducer: Arc::new(RwLock::new(reducer)) }
	}

	/// Get current reducer state and heads.
	pub async fn reducer_state(&self) -> (Option<Cid>, BTreeSet<Cid>) {
		let reducer = self.reducer.read().await;
		(reducer.state().clone(), reducer.heads().clone())
	}

	/// Get storage instance for this CO.
	pub fn storage(&self) -> CoStorage {
		self.storage.clone()
	}

	/// Push event into reducer.
	pub async fn push<T, I>(&self, identity: &I, co: &str, item: &T) -> Result<(), anyhow::Error>
	where
		T: Serialize + Send + Sync + Clone + 'static,
		I: PrivateIdentity + Send + Sync,
	{
		self.reducer
			.write()
			.await
			.push(self.runtime.runtime(), identity, co, item)
			.await
	}

	/// Read co reducer state.
	pub async fn co(&self) -> Result<co_core_co::Co, CoReducerError> {
		let (storage, state) = {
			let reducer = self.reducer.read().await;
			(reducer.log().storage().clone(), reducer.state().clone())
		};
		if let Some(state_cid) = state {
			return Ok(storage.get_deserialized(&state_cid).await?)
		}
		return Ok(co_core_co::Co::default());
	}

	/// Read a COre state.
	pub async fn state<T: DeserializeOwned + Send + Sync + Default + Clone + 'static>(
		&self,
		co: &str,
	) -> Result<T, CoReducerError> {
		let (storage, state) = {
			let reducer = self.reducer.read().await;
			(reducer.log().storage().clone(), reducer.state().clone())
		};

		// co?
		if co == CO_CORE_NAME_CO {
			if let Some(state_cid) = state {
				return Ok(storage.get_deserialized(&state_cid).await?)
			}
			return Ok(T::default());
		}

		// other
		let co_state: co_core_co::Co = if let Some(state_cid) = state {
			storage.get_deserialized(&state_cid).await?
		} else {
			co_core_co::Co::default()
		};
		if let Some(core) = co_state.cores.get(co) {
			if let Some(core_state) = &core.state {
				return Ok(storage.get_deserialized(core_state).await?);
			} else {
				return Ok(T::default());
			}
		}

		// not found
		return Err(CoReducerError::CoreNotFound(co.to_owned()));
	}
}

#[derive(Debug, thiserror::Error)]
pub enum CoReducerError {
	#[error("Storage error")]
	Storage(#[from] StorageError),

	#[error("Core not found: {0}")]
	CoreNotFound(String),
}

// pub type CoReducer = Reducer<EncryptedBlockStorage<CoStorage>, CoCoreResolver<EncryptedBlockStorage<CoStorage>>>;
