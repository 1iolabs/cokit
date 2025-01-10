use super::co_storage::CoBlockStorageContentMapping;
use crate::{reducer::core_resolver::dynamic::DynamicCoreResolver, state::core_state, CoStorage, Reducer, Runtime};
use async_trait::async_trait;
use co_identity::PrivateIdentity;
use co_primitives::{CoId, KnownMultiCodec, OptionLink, ReducerAction};
use co_storage::{BlockStorageContentMapping, BlockStorageExt, MappedBlockStorage, StorageError};
use futures::{stream, StreamExt, TryStreamExt};
use libipld::Cid;
use serde::{de::DeserializeOwned, Serialize};
use std::{collections::BTreeSet, fmt::Debug, sync::Arc};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct CoReducer {
	id: CoId,
	parent: Option<CoId>,
	pub(crate) reducer: Arc<RwLock<Reducer<CoStorage, DynamicCoreResolver<CoStorage>>>>,
	pub(crate) storage: CoStorage,
	pub(crate) runtime: Runtime,
	pub(crate) context: Arc<dyn CoReducerContext + Send + Sync + 'static>,
}
impl CoReducer {
	pub(crate) fn new(
		id: CoId,
		parent: Option<CoId>,
		runtime: Runtime,
		reducer: Reducer<CoStorage, DynamicCoreResolver<CoStorage>>,
		context: Arc<dyn CoReducerContext + Send + Sync + 'static>,
	) -> Self {
		Self {
			id,
			parent,
			runtime,
			storage: reducer.log().storage().clone(),
			reducer: Arc::new(RwLock::new(reducer)),
			context,
		}
	}

	pub fn id(&self) -> &CoId {
		&self.id
	}

	pub fn parent_id(&self) -> Option<&CoId> {
		self.parent.as_ref()
	}

	/// Get current reducer heads.
	pub async fn heads(&self) -> BTreeSet<Cid> {
		let reducer = self.reducer.read().await;
		reducer.heads().clone()
	}

	/// Get current reducer state and heads.
	pub async fn reducer_state(&self) -> (Option<Cid>, BTreeSet<Cid>) {
		let reducer = self.reducer.read().await;
		(*reducer.state(), reducer.heads().clone())
	}

	/// Get storage instance for this CO.
	pub fn storage(&self) -> CoStorage {
		self.storage.clone()
	}

	/// Get mapped storage instance for this CO.
	pub fn mapped_storage(&self) -> CoStorage {
		let storage = self.storage();
		if let Some(mapping) = self.context.content_mapping() {
			CoStorage::new(MappedBlockStorage::new(
				storage,
				mapping,
				[KnownMultiCodec::CoEncryptedBlock.into()].into_iter().collect(),
			))
		} else {
			storage
		}
	}

	/// Get reducer watcher.
	pub async fn watch(&self) -> tokio::sync::watch::Receiver<Option<(Cid, BTreeSet<Cid>)>> {
		self.reducer.read().await.watch()
	}

	/// Push event into reducer.
	///
	/// # Arguments
	/// - `identity` - The identity to sign the operation with.
	/// - `core` - The target core name. The key of [`co_core_co::Co::cores`].
	/// - `item` - The core action payload.
	#[tracing::instrument(err, ret, name = "push", fields(co = self.id().as_str(), identity = identity.identity()), skip(self, item, identity))]
	pub async fn push<T, I>(&self, identity: &I, core: &str, item: &T) -> Result<Option<Cid>, anyhow::Error>
	where
		T: Serialize + Debug + Send + Sync + Clone + 'static,
		I: PrivateIdentity + Send + Sync,
	{
		tracing::trace!(action = ?item, "push");
		self.reducer
			.write()
			.await
			.push(self.runtime.runtime(), identity, core, item)
			.await
	}

	/// Push event into reducer.
	#[tracing::instrument(err, ret, name = "push", fields(co = self.id().as_str(), identity = identity.identity(), core = action.core), skip(self, action, identity))]
	pub async fn push_action<T, I>(&self, identity: &I, action: &ReducerAction<T>) -> Result<Option<Cid>, anyhow::Error>
	where
		T: Serialize + Debug + Send + Sync + Clone + 'static,
		I: PrivateIdentity + Send + Sync,
	{
		tracing::trace!(action = ?action.payload, "push");
		self.reducer
			.write()
			.await
			.push_action(self.runtime.runtime(), identity, action)
			.await
	}

	/// Join heads.
	/// Returns true if state has changed.
	#[tracing::instrument(err, ret, fields(co = self.id().as_str()), skip(self))]
	pub async fn join(&self, heads: &BTreeSet<Cid>) -> Result<bool, anyhow::Error> {
		// to internal cids
		let internal_heads: BTreeSet<Cid> = stream::iter(heads.iter())
			.then(|cid| async { self.context.to_internal_cid(*cid).await })
			.try_collect()
			.await?;

		// join
		Ok(self.reducer.write().await.join(&internal_heads, self.runtime.runtime()).await?)
	}

	/// Insert a previous (trusted) snapshot into histroy which may can used as a starting point.
	pub async fn insert_snapshot(&self, state: Cid, heads: BTreeSet<Cid>) -> Result<(), StorageError> {
		// to internal cids
		let internal_state = self.context.to_internal_cid(state).await?;
		let internal_heads: BTreeSet<Cid> = stream::iter(heads.iter())
			.then(|cid| async { self.context.to_internal_cid(*cid).await })
			.try_collect()
			.await?;

		// insert
		self.reducer.write().await.insert_snapshot(internal_state, internal_heads);

		// result
		Ok(())
	}

	/// Read co reducer state.
	pub async fn co(&self) -> Result<co_core_co::Co, CoReducerError> {
		let (storage, state) = {
			let reducer = self.reducer.read().await;
			(reducer.log().storage().clone(), *reducer.state())
		};
		if let Some(state_cid) = state {
			return Ok(storage.get_deserialized(&state_cid).await?);
		}
		Ok(co_core_co::Co::default())
	}

	/// Read co reducer state reference.
	pub async fn co_state(&self) -> OptionLink<co_core_co::Co> {
		let reducer = self.reducer.read().await;
		reducer.state().into()
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
	) -> Option<(
		CoStorage,
		Reducer<CoStorage, DynamicCoreResolver<CoStorage>>,
		Arc<dyn CoReducerContext + Send + Sync + 'static>,
	)> {
		Arc::into_inner(self.reducer).map(|lock| (self.storage, lock.into_inner(), self.context))
	}

	/// Convert an CO CID to an external (plain) CID.
	pub async fn to_external_cid(&self, cid: Cid) -> Cid {
		match &self.context.content_mapping() {
			Some(mapping) => mapping.to_plain(&cid).await.unwrap_or(cid),
			None => cid,
		}
	}

	/// Get current reducer state and heads.
	pub async fn external_reducer_state(&self) -> (Option<Cid>, BTreeSet<Cid>) {
		let (state, heads) = self.reducer_state().await;
		(
			match state {
				Some(cid) => Some(self.to_external_cid(cid).await),
				None => None,
			},
			stream::iter(heads.into_iter())
				.then(|cid| async move { self.to_external_cid(cid).await })
				.collect()
				.await,
		)
	}

	/// Refresh the reducer instance parent state.
	pub async fn refresh(&self, parent: CoReducer) -> anyhow::Result<()> {
		self.context.refresh(parent, self.clone()).await
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

#[async_trait]
pub trait CoReducerContext {
	/// Get encryption mapping instance.
	fn content_mapping(&self) -> Option<CoBlockStorageContentMapping>;

	/// Refresh reducer instance state from source.
	async fn refresh(&self, parent: CoReducer, co: CoReducer) -> anyhow::Result<()>;

	/// Map external [`Cid`] to internal [`Cid`].
	async fn to_internal_cid(&self, cid: Cid) -> Result<Cid, StorageError>;

	/// Map internal [`Cid`] to external [`Cid`].
	async fn to_external_cid(&self, cid: Cid) -> Result<Cid, StorageError>;
}
