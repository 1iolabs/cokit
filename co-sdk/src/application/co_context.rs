use super::{application::ApplicationSettings, shared::SharedCoBuilder};
use crate::{
	drivers::network::{token::CoToken, CoNetworkTaskSpawner},
	library::{find_co_secret::find_co_secret, find_membership::find_membership},
	reducer::core_resolver::membership::{MembershipCoreResolver, MembershipInstanceRegistry},
	types::co_reducer::CoReducerContext,
	CoCoreResolver, CoReducer, CoReducerFactory, CoStorage, LocalCoBuilder, Runtime, TaskSpawner,
	CO_CORE_NAME_KEYSTORE, CO_CORE_NAME_MEMBERSHIP, CO_ID_LOCAL,
};
use anyhow::anyhow;
use async_trait::async_trait;
use co_identity::{LocalIdentity, PrivateIdentity};
use co_log::EntryBlock;
use co_network::bitswap;
use co_primitives::CoId;
use co_storage::BlockStorage;
use futures::{Stream, TryStreamExt};
use libp2p::PeerId;
use std::{collections::BTreeMap, fmt::Debug, sync::Arc};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

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
	/// Starting at the latest (reverse chronological).
	/// The stream is read with snapshot isolation (not watching changes).
	pub async fn entries(
		&self,
		co: impl AsRef<CoId>,
	) -> Result<
		(
			CoStorage,
			impl Stream<Item = Result<EntryBlock<<CoStorage as BlockStorage>::StoreParams>, anyhow::Error>>,
			Arc<dyn CoReducerContext + Send + Sync + 'static>,
		),
		anyhow::Error,
	> {
		let co = co.as_ref();

		// create
		let initialized = true;
		let uninitialized_reducer = if co.as_str() == CO_ID_LOCAL {
			self.inner.create_local_co_instance(initialized).await?
		} else {
			let local = self.local_co_reducer().await?;
			self.inner
				.create_co_instance(local, co, initialized, self.inner.local_identity.clone())
				.await?
				.ok_or(anyhow!("Co not found: {}", co))?
		};
		let (storage, reducer, context) = uninitialized_reducer.into_inner().ok_or(anyhow!("Invalid reference"))?;
		let log = reducer.into_log();

		// stream
		let stream = log.into_stream().map_err(|e| e.into());

		// result
		Ok((storage, stream, context))
	}
}
#[async_trait]
impl CoReducerFactory for CoContext {
	async fn co_reducer(&self, co: &CoId) -> Result<Option<CoReducer>, anyhow::Error> {
		self.inner.co_reducer(co).await
	}
}
#[async_trait]
impl bitswap::StorageResolver<CoStorage> for CoContext {
	async fn resolve_storage(
		&self,
		remote_peer: Option<&PeerId>,
		tokens: &[bitswap::Token],
	) -> Result<CoStorage, anyhow::Error> {
		// use CO from first valid token
		for token in tokens {
			if let Ok(co_token) = CoToken::from_bitswap_token(token) {
				// get co
				if let Some(co) = self.co_reducer(&co_token.body.1).await? {
					let parent = match co.parent_id() {
						Some(id) => self.co_reducer(id).await?.ok_or(anyhow!("Unknown CO: {}", id)),
						None => Err(anyhow!("Unsupported CO: {}", co_token.body.1)),
					}?;
					let secret = find_co_secret(&parent, &co).await?;

					// verify remote peer if the CO is encrypted and this is an non local request
					match (remote_peer, &secret) {
						(Some(remote_peer), Some(secret)) => {
							if !co_token.verify(secret, remote_peer) {
								// check next token
								continue;
							}
						},
						_ => {},
					};

					// get storage
					return Ok(co.storage());
				}
			}
		}

		// use the root storage (unencrypted)
		Ok(self.inner.storage())
	}
}

#[derive(Clone)]
pub(crate) struct CoContextInner {
	settings: ApplicationSettings,

	shutdown: CancellationToken,
	tasks: TaskSpawner,

	local_identity: LocalIdentity,

	network: Arc<RwLock<Option<CoNetworkTaskSpawner>>>,
	storage: CoStorage,
	runtime: Runtime,

	/// Loaded reducers.
	reducers: Arc<RwLock<BTreeMap<CoId, CoReducer>>>,
}
impl CoContextInner {
	pub(crate) fn new(
		settings: ApplicationSettings,
		shutdown: CancellationToken,
		tasks: TaskSpawner,
		local_identity: LocalIdentity,
		network: Option<CoNetworkTaskSpawner>,
		storage: CoStorage,
		runtime: Runtime,
	) -> Self {
		Self {
			settings,
			shutdown,
			tasks,
			local_identity,
			network: Arc::new(RwLock::new(network)),
			storage,
			runtime,
			reducers: Default::default(),
		}
	}

	/// Get the root storage.
	pub fn storage(&self) -> CoStorage {
		self.storage.clone()
	}

	/// Clone with network.
	pub async fn set_network(&self, network: Option<CoNetworkTaskSpawner>) {
		// assign
		*self.network.write().await = network;

		// clear reducers
		self.reducers.write().await.retain(|id, _reducer| id.as_str() == CO_ID_LOCAL);
	}

	/// Get instance of Local CoReducer.
	pub async fn local_co_reducer(&self) -> Result<CoReducer, anyhow::Error> {
		let co = CO_ID_LOCAL;

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
		let core_resolver = MembershipCoreResolver::new(
			self.tasks.clone(),
			CoCoreResolver::default(),
			CoContextMembershipInstanceRegistry { reducers: self.reducers.clone() },
			CO_CORE_NAME_MEMBERSHIP.to_owned(),
		);
		let local_co = LocalCoBuilder::new(self.settings.clone(), self.local_identity.clone(), initialize);
		let local_co_reducer = local_co
			.build(
				self.storage.clone(),
				self.runtime.clone(),
				self.shutdown.child_token(),
				self.tasks.clone(),
				core_resolver,
			)
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
		let reducer = if co.as_str() == CO_ID_LOCAL {
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
			.with_network(self.network.read().await.clone())
			.with_initialize(initialize)
			.build(self.tasks.clone(), self.storage.clone(), self.runtime.clone(), identity)
			.await?;
		Ok(Some(reducer))
	}
}
impl From<CoContextInner> for CoContext {
	fn from(val: CoContextInner) -> Self {
		CoContext { inner: Arc::new(val) }
	}
}

#[derive(Clone)]
struct CoContextMembershipInstanceRegistry {
	reducers: Arc<RwLock<BTreeMap<CoId, CoReducer>>>,
}
#[async_trait]
impl MembershipInstanceRegistry for CoContextMembershipInstanceRegistry {
	async fn update(&self, co: CoId) -> Result<(), anyhow::Error> {
		if let Some(co_reducer) = self.reducers.read().await.get(&co).cloned() {
			if let Some(parent) = co_reducer.parent_id() {
				if let Some(parent_co_reducer) = self.reducers.read().await.get(parent).cloned() {
					let context = co_reducer.context.clone();
					context.refresh(parent_co_reducer, co_reducer).await?;
				}
			}
		}
		Ok(())
	}

	async fn remove(&self, co: CoId) -> Result<(), anyhow::Error> {
		self.reducers.write().await.remove(&co);
		Ok(())
	}
}
