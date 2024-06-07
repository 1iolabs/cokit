use super::{application::ApplicationSettings, shared::SharedCoBuilder};
use crate::{
	drivers::network::CoNetworkTaskSpawner, library::find_membership::find_membership,
	types::co_storage::CoBlockStorageContentMapping, CoReducer, CoReducerFactory, CoStorage, LocalCoBuilder, Runtime,
	CO_CORE_NAME_KEYSTORE, CO_CORE_NAME_MEMBERSHIP,
};
use anyhow::anyhow;
use async_trait::async_trait;
use co_identity::{LocalIdentity, PrivateIdentity};
use co_log::EntryBlock;
use co_primitives::CoId;
use co_storage::BlockStorage;
use futures::{Stream, TryStreamExt};
use std::{collections::BTreeMap, fmt::Debug, sync::Arc};
use tokio::sync::RwLock;
use tokio_util::{sync::CancellationToken, task::TaskTracker};

#[derive(Clone)]
pub struct CoContext {
	pub(crate) inner: Arc<CoContextInner>,
}
impl CoContext {
	/// Get instance of Local CoReducer.
	pub async fn local_co_reducer(&self) -> Result<CoReducer, anyhow::Error> {
		self.inner.local_co_reducer().await
	}

	/// Get a stream to the log entries.
	/// Starting at the latest.
	pub async fn entries(
		&self,
		co: impl AsRef<CoId>,
	) -> Result<
		(
			CoStorage,
			impl Stream<Item = Result<EntryBlock<<CoStorage as BlockStorage>::StoreParams>, anyhow::Error>>,
			Option<CoBlockStorageContentMapping>,
		),
		anyhow::Error,
	> {
		let co = co.as_ref();

		// create
		let initialized = true;
		let uninitialized_reducer = if co.as_str() == "local" {
			self.inner.create_local_co_instance(initialized).await?
		} else {
			let local = self.local_co_reducer().await?;
			self.inner
				.create_co_instance(local, co, initialized, self.inner.local_identity.clone())
				.await?
				.ok_or(anyhow!("Co not found: {}", co))?
		};
		let (storage, reducer, mapping) = uninitialized_reducer.into_inner().ok_or(anyhow!("Invalid reference"))?;
		let log = reducer.into_log();

		// stream
		let stream = log.into_stream().map_err(|e| e.into());

		// result
		Ok((storage, stream, mapping))
	}
}
#[async_trait]
impl CoReducerFactory for CoContext {
	async fn co_reducer(&self, co: &CoId) -> Result<Option<CoReducer>, anyhow::Error> {
		self.inner.co_reducer(co).await
	}
}

pub(crate) struct CoContextInner {
	settings: ApplicationSettings,

	shutdown: CancellationToken,
	tasks: TaskTracker,

	local_identity: LocalIdentity,

	network: Option<CoNetworkTaskSpawner>,
	storage: CoStorage,
	runtime: Runtime,

	/// Loaded reducers.
	reducers: RwLock<BTreeMap<CoId, CoReducer>>,
}
impl CoContextInner {
	pub(crate) fn new(
		settings: ApplicationSettings,
		shutdown: CancellationToken,
		tasks: TaskTracker,
		local_identity: LocalIdentity,
		network: Option<CoNetworkTaskSpawner>,
		storage: CoStorage,
		runtime: Runtime,
	) -> Self {
		Self { settings, shutdown, tasks, local_identity, network, storage, runtime, reducers: Default::default() }
	}

	/// Clone with network.
	pub async fn with_network(&self, network: Option<CoNetworkTaskSpawner>) -> Self {
		Self {
			settings: self.settings.clone(),
			shutdown: self.shutdown.clone(),
			tasks: self.tasks.clone(),
			local_identity: self.local_identity.clone(),
			network,
			storage: self.storage.clone(),
			runtime: self.runtime.clone(),
			reducers: {
				// we only keep local as this has no network
				let mut next_reducers = BTreeMap::new();
				let reducers = self.reducers.read().await;
				if let Some(local) = reducers.get("local") {
					next_reducers.insert("local".into(), local.clone());
				}
				RwLock::new(next_reducers)
			},
		}
	}

	/// Get instance of Local CoReducer.
	pub async fn local_co_reducer(&self) -> Result<CoReducer, anyhow::Error> {
		let co = "local";

		// has one?
		{
			let reducers = self.reducers.read().await;
			let reducer = reducers.get(co);
			if let Some(reducer) = reducer {
				return Ok(reducer.clone());
			}
		}

		// create
		let reducer = self.create_local_co_instance(true).await?;

		// store
		self.reducers.write().await.insert(co.into(), reducer.clone());

		// result
		Ok(reducer)
	}

	/// Creates a CoReducer instance of the Local CO.
	async fn create_local_co_instance(&self, initialize: bool) -> Result<CoReducer, anyhow::Error> {
		let local_co = LocalCoBuilder::new(self.settings.clone(), self.local_identity.clone(), initialize);
		let local_co_reducer = local_co
			.build(self.storage.clone(), self.runtime.clone(), self.shutdown.child_token(), self.tasks.clone())
			.await?;
		Ok(local_co_reducer)
	}

	/// Get instance of CoReducer.
	/// Returns None if `co` membership could not be found.
	///
	/// TODO: Identity
	///   - Which identity should write to the parent co? If its local we are fine.
	pub async fn co_reducer(&self, co: impl AsRef<CoId>) -> Result<Option<CoReducer>, anyhow::Error> {
		let co = co.as_ref();

		// has one?
		{
			let reducers = self.reducers.read().await;
			let reducer = reducers.get(co);
			if let Some(reducer) = reducer {
				return Ok(Some(reducer.clone()));
			}
		}

		// create
		let reducer = if co.as_str() == "local" {
			Some(self.create_local_co_instance(true).await?)
		} else {
			let local = self.local_co_reducer().await?;
			self.create_co_instance(local, co, true, self.local_identity.clone()).await?
		};

		// store
		if let Some(reducer_cache) = &reducer {
			self.reducers.write().await.insert(co.to_owned(), reducer_cache.clone());
		}

		// result
		Ok(reducer)
	}

	/// Creates a CoReducer instance a CO which we have a membership for.
	async fn create_co_instance<I>(
		&self,
		parent: CoReducer,
		co: &CoId,
		initialize: bool,
		identity: I,
	) -> Result<Option<CoReducer>, anyhow::Error>
	where
		I: PrivateIdentity + Debug + Send + Sync + Clone + 'static,
	{
		let membership = match find_membership(&parent, co).await? {
			Some(m) => m,
			None => return Ok(None),
		};
		let reducer = SharedCoBuilder::new(parent, membership)
			.with_membership_core_name(CO_CORE_NAME_MEMBERSHIP.to_owned())
			.with_keystore_core_name(CO_CORE_NAME_KEYSTORE.to_owned())
			.with_network(self.network.clone())
			.with_initialize(initialize)
			.build(self.storage.clone(), self.runtime.clone(), identity)
			.await?;
		Ok(Some(reducer))
	}
}
impl Into<CoContext> for CoContextInner {
	fn into(self) -> CoContext {
		CoContext { inner: Arc::new(self) }
	}
}
