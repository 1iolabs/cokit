use crate::{CoCoreResolver, CoStorage, Cores, Reducer, Runtime, CO_CORE_CO};
use anyhow::anyhow;
use co_storage::{BlockStorageExt, EncryptedBlockStorage};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct CoReducer {
	pub(crate) reducer:
		Arc<RwLock<Reducer<EncryptedBlockStorage<CoStorage>, CoCoreResolver<EncryptedBlockStorage<CoStorage>>>>>,
	pub(crate) runtime: Arc<Runtime>,
}

impl CoReducer {
	/// Push event into reducer.
	pub async fn push<T: Serialize + Send + Sync + Clone + 'static>(
		&self,
		co: &str,
		item: &T,
	) -> Result<(), anyhow::Error> {
		self.reducer.write().await.push(self.runtime.runtime(), co, item).await
	}

	/// Read co reducer state.
	pub async fn co(&self) -> Result<co_core_co::Co, anyhow::Error> {
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
	) -> Result<T, anyhow::Error> {
		let (storage, state) = {
			let reducer = self.reducer.read().await;
			(reducer.log().storage().clone(), reducer.state().clone())
		};

		// co?
		if co == Cores::to_core_name(CO_CORE_CO) {
			if let Some(state_cid) = state {
				return Ok(storage.get_deserialized(&state_cid).await?)
			}
			return Ok(T::default());
		}

		// other
		let co_state: co_core_co::Co = if let Some(state_cid) = state {
			return Ok(storage.get_deserialized(&state_cid).await?)
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
		return Err(anyhow!("Core not found: {}", co));
	}
}

// pub type CoReducer = Reducer<EncryptedBlockStorage<CoStorage>, CoCoreResolver<EncryptedBlockStorage<CoStorage>>>;
