use super::{co_dispatch::CoDispatch, co_storage::CoBlockStorageContentMapping};
use crate::{
	library::to_external_cid::{to_external_cid, to_external_cid_opt, to_external_cids},
	reducer::core_resolver::dynamic::DynamicCoreResolver,
	state::core_state,
	CoStorage, Reducer, Runtime,
};
use async_trait::async_trait;
use cid::Cid;
use co_core_co::Co;
use co_identity::{PrivateIdentity, PrivateIdentityBox};
use co_primitives::{
	BlockStorageSettings, CloneWithBlockStorageSettings, CoId, KnownMultiCodec, OptionLink, ReducerAction,
};
use co_storage::{BlockStorageExt, MappedBlockStorage, StorageError};
use futures::{stream, Stream, StreamExt, TryStreamExt};
use serde::{de::DeserializeOwned, Serialize};
use std::{collections::BTreeSet, fmt::Debug, marker::PhantomData, sync::Arc};
use tokio::sync::RwLock;
use tokio_stream::wrappers::WatchStream;

#[derive(Clone)]
pub struct CoReducer {
	id: CoId,
	parent: Option<CoId>,
	pub(crate) reducer: Arc<RwLock<Reducer<CoStorage, DynamicCoreResolver<CoStorage>>>>,
	pub(crate) storage: CoStorage,
	pub(crate) runtime: Runtime,
	pub(crate) context: CoReducerContextRef,
}
impl CoReducer {
	pub(crate) fn new(
		id: CoId,
		parent: Option<CoId>,
		runtime: Runtime,
		reducer: Reducer<CoStorage, DynamicCoreResolver<CoStorage>>,
		context: CoReducerContextRef,
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

	/// Get a new storage instance for this CO.
	/// This starts a new session and the same (or cloned) storage instance should be used while traversing the co state.
	pub fn storage(&self) -> CoStorage {
		self.storage.clone_with_settings(BlockStorageSettings::new().with_detached())
	}

	/// Get mapped storage instance for this CO.
	#[deprecated]
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

	/// Get reducer change stream.
	/// Upon start (if the reducer is not empty) the current state is yielded.
	pub fn reducer_state_stream(&self) -> impl Stream<Item = (Cid, BTreeSet<Cid>)> + use<'_> {
		async_stream::stream! {
			let stream = WatchStream::new(self.reducer.read().await.watch());
			for await change in stream {
				if let Some(change) = change {
					yield change;
				}
			}
		}
	}

	/// Push event into reducer.
	///
	/// # Arguments
	/// - `identity` - The identity to sign the operation with.
	/// - `core` - The target core name. The key of [`co_core_co::Co::cores`].
	/// - `item` - The core action payload.
	///
	/// # Returns
	/// The resulting state.
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
	pub async fn co(&self) -> Result<Co, CoReducerError> {
		Ok(self.storage().get_value(&self.co_state().await).await?)
	}

	/// Read co reducer state reference.
	pub async fn co_state(&self) -> OptionLink<Co> {
		let reducer = self.reducer.read().await;
		reducer.state().into()
	}

	/// Read a COre state.
	///
	/// # Arguments
	/// - `core` - The core name.
	#[deprecated(note = "please use `query_core`")]
	pub async fn state<T: DeserializeOwned + Send + Sync + Default + Clone + 'static>(
		&self,
		core: &str,
	) -> Result<T, CoReducerError> {
		Ok(core_state(&self.storage(), self.co_state().await, core).await?.1)
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
		match self.context.content_mapping() {
			Some(mapping) => to_external_cid(&mapping, cid).await,
			None => cid,
		}
	}

	/// Get current reducer state and heads.
	pub async fn external_reducer_state(&self) -> (Option<Cid>, BTreeSet<Cid>) {
		let (state, heads) = self.reducer_state().await;
		if let Some(mapping) = self.context.content_mapping() {
			(to_external_cid_opt(&mapping, state).await, to_external_cids(&mapping, heads).await)
		} else {
			(state, heads)
		}
	}

	/// Refresh the reducer instance parent state.
	pub async fn refresh(&self, parent: CoReducer) -> anyhow::Result<()> {
		self.context.refresh(parent, self.clone()).await
	}

	/// Create a action dispatcher.
	pub fn dispatcher<A, I>(&self, core: &str, identity: I) -> impl CoDispatch<A> + use<A, I>
	where
		A: Serialize + Debug + Send + Sync + Clone + 'static,
		I: PrivateIdentity + Debug + Clone + Send + Sync + 'static,
	{
		CoReducerDispatch::new(self.clone(), PrivateIdentity::boxed(identity), core.to_string())
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
pub trait CoReducerContext: Debug {
	/// Get a new storage instance.
	///
	/// # Args
	/// - `force_local` - If true the new instance should not use networking.
	fn storage(&self, force_local: bool) -> CoStorage;

	/// Get encryption mapping instance.
	fn content_mapping(&self) -> Option<CoBlockStorageContentMapping>;

	/// Refresh reducer instance state from source.
	async fn refresh(&self, parent: CoReducer, co: CoReducer) -> anyhow::Result<()>;

	/// Map external [`Cid`] to internal [`Cid`].
	async fn to_internal_cid(&self, cid: Cid) -> Result<Cid, StorageError>;

	/// Map internal [`Cid`] to external [`Cid`].
	async fn to_external_cid(&self, cid: Cid) -> Result<Cid, StorageError>;
}

pub type CoReducerContextRef = Arc<dyn CoReducerContext + Send + Sync + 'static>;

struct CoReducerDispatch<A> {
	reducer: CoReducer,
	identity: PrivateIdentityBox,
	core: String,
	_action: PhantomData<A>,
}
impl<A> CoReducerDispatch<A> {
	fn new(reducer: CoReducer, identity: PrivateIdentityBox, core: String) -> Self {
		Self { reducer, identity, core, _action: Default::default() }
	}
}
#[async_trait]
impl<A> CoDispatch<A> for CoReducerDispatch<A>
where
	A: Serialize + Debug + Send + Sync + Clone + 'static,
{
	async fn dispatch(&self, action: &A) -> Result<Option<Cid>, anyhow::Error> {
		Ok(self.reducer.push(&self.identity, &self.core, action).await?)
	}
}
