use super::co_storage::CoBlockStorageContentMapping;
use crate::{CoCoreResolver, CoStorage, Reducer, Runtime, CO_CORE_NAME_CO};
use co_identity::PrivateIdentity;
use co_primitives::CoId;
use co_storage::{BlockStorageExt, StorageError};
use libipld::Cid;
use serde::{de::DeserializeOwned, Serialize};
use std::{collections::BTreeSet, sync::Arc};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct CoReducer {
	id: CoId,
	pub(crate) reducer: Arc<RwLock<Reducer<CoStorage, CoCoreResolver>>>,
	pub(crate) storage: CoStorage,
	pub(crate) runtime: Runtime,
	pub(crate) mapping: Option<CoBlockStorageContentMapping>,
}
impl CoReducer {
	pub(crate) fn new(
		id: CoId,
		runtime: Runtime,
		reducer: Reducer<CoStorage, CoCoreResolver>,
		mapping: Option<CoBlockStorageContentMapping>,
	) -> Self {
		Self { id, runtime, storage: reducer.log().storage().clone(), reducer: Arc::new(RwLock::new(reducer)), mapping }
	}

	pub fn id(&self) -> &CoId {
		&self.id
	}

	/// Get current reducer heads.
	pub async fn heads(&self) -> BTreeSet<Cid> {
		let reducer = self.reducer.read().await;
		reducer.heads().clone()
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

	/// Join heads.
	/// Returns true if state has changed.
	pub async fn join(&self, heads: BTreeSet<Cid>) -> Result<bool, anyhow::Error> {
		Ok(self.reducer.write().await.join(&heads, self.runtime.runtime()).await?)
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
		core: &str,
	) -> Result<T, CoReducerError> {
		let (storage, state) = {
			let reducer = self.reducer.read().await;
			(reducer.log().storage().clone(), reducer.state().clone())
		};

		// co?
		if core == CO_CORE_NAME_CO {
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
		if let Some(core) = co_state.cores.get(core) {
			if let Some(core_state) = &core.state {
				return Ok(storage.get_deserialized(core_state).await?);
			} else {
				return Ok(T::default());
			}
		}

		// not found
		return Err(CoReducerError::CoreNotFound(core.to_owned()));
	}

	/// Try to escape inner data.
	pub(crate) fn into_inner(self) -> Option<(CoStorage, Reducer<CoStorage, CoCoreResolver>)> {
		Arc::into_inner(self.reducer).map(|lock| (self.storage, lock.into_inner()))
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
