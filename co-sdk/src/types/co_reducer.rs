use super::{co_storage::CoBlockStorageContentMapping, state_observable::StateObservable};
use crate::{state::core_state, CoCoreResolver, CoStorage, Reducer, Runtime};
use co_identity::PrivateIdentity;
use co_primitives::CoId;
use co_storage::{BlockStorageExt, StorageError};
use libipld::Cid;
use serde::{de::DeserializeOwned, Serialize};
use std::{collections::BTreeSet, fmt::Debug, sync::Arc};
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

	/// Get reducer observable.
	#[deprecated]
	pub async fn observable(&self) -> StateObservable {
		StateObservable { sub: self.reducer.read().await.observable() }
	}

	/// Get reducer watcher.
	pub async fn watch(&self) -> tokio::sync::watch::Receiver<Option<(Cid, BTreeSet<Cid>)>> {
		self.reducer.read().await.watch()
	}

	/// Push event into reducer.
	#[tracing::instrument(err, fields(co = self.id().as_str()), skip(self))]
	pub async fn push<T, I>(&self, identity: &I, core: &str, item: &T) -> Result<(), anyhow::Error>
	where
		T: Serialize + Debug + Send + Sync + Clone + 'static,
		I: PrivateIdentity + Debug + Send + Sync,
	{
		self.reducer
			.write()
			.await
			.push(self.runtime.runtime(), identity, core, item)
			.await
	}

	/// Join heads.
	/// Returns true if state has changed.
	#[tracing::instrument(err, ret, fields(co = self.id().as_str()), skip(self))]
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
	///
	/// # Arguments
	/// - `core` - The core name.
	pub async fn state<T: DeserializeOwned + Send + Sync + Default + Clone + 'static>(
		&self,
		core: &str,
	) -> Result<T, CoReducerError> {
		let (storage, state) = {
			let reducer = self.reducer.read().await;
			(reducer.log().storage().clone(), *reducer.state())
		};
		Ok(core_state(&storage, state.into(), core).await?.1)
	}

	/// Try to escape inner data.
	pub(crate) fn into_inner(
		self,
	) -> Option<(CoStorage, Reducer<CoStorage, CoCoreResolver>, Option<CoBlockStorageContentMapping>)> {
		Arc::into_inner(self.reducer).map(|lock| (self.storage, lock.into_inner(), self.mapping))
	}
}
impl Debug for CoReducer {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("CoReducer")
			.field("id", &self.id)
			//.field("reducer", &self.reducer)
			//.field("storage", &self.storage)
			//.field("runtime", &self.runtime)
			//.field("mapping", &self.mapping)
			.finish()
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
