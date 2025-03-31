#[cfg(feature = "pinning")]
use crate::reducer::core_resolver::change::ChangeCoreResolver;
#[cfg(feature = "pinning")]
use crate::reducer::core_resolver::reference::ReferenceCoreResolver;
#[cfg(feature = "pinning")]
use crate::types::co_pinning_key::CoPinningKey;
use crate::{
	application::{
		application::ApplicationSettings,
		identity::{create_identity_resolver, create_private_identity_resolver},
		shared::SharedCoBuilder,
	},
	library::shared_membership::shared_membership,
	reducer::core_resolver::{dynamic::DynamicCoreResolver, epic::ReactiveCoreResolver, log::LogCoreResolver},
	services::{
		application::ApplicationMessage,
		connections::ConnectionMessage,
		network::CoNetworkTaskSpawner,
		reducers::{ReducerStorage, ReducersControl},
	},
	types::co_reducer_factory::CoReducerFactoryError,
	CoCoreResolver, CoReducer, CoReducerFactory, CoStorage, LocalCoBuilder, Runtime, TaskSpawner,
	CO_CORE_NAME_KEYSTORE, CO_CORE_NAME_MEMBERSHIP, CO_CORE_NAME_STORAGE, CO_ID_LOCAL,
};
use async_trait::async_trait;
use co_actor::ActorHandle;
use co_core_membership::Membership;
use co_identity::{
	IdentityResolverBox, LocalIdentity, PrivateIdentity, PrivateIdentityResolver, PrivateIdentityResolverBox,
};
use co_log::{EntryBlock, Log};
use co_primitives::{BlockStorageSettings, CloneWithBlockStorageSettings, CoId, Did};
#[cfg(feature = "pinning")]
use co_storage::ChangeBlockStorage;
use futures::{Stream, TryStreamExt};
use std::{
	fmt::Debug,
	sync::{Arc, RwLock},
};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct CoContext {
	pub(crate) inner: Arc<CoContextInner>,
}
impl CoContext {
	/// Get instance of Local CoReducer.
	#[tracing::instrument(skip(self), fields(application = self.inner.settings.identifier))]
	pub async fn local_co_reducer(&self) -> Result<CoReducer, anyhow::Error> {
		Ok(self.inner.reducers.clone().reducer(CoId::from(CO_ID_LOCAL)).await?)
	}

	/// Get a stream to the log entries.
	/// Starting at the latest (reverse chronological).
	/// The stream is read with snapshot isolation (not watching changes).
	pub async fn entries(
		&self,
		co: impl AsRef<CoId>,
	) -> Result<(CoStorage, impl Stream<Item = Result<EntryBlock, anyhow::Error>>), anyhow::Error> {
		let co = co.as_ref();

		// log
		let reducer = self.try_co_reducer(&co).await?;
		let storage = reducer.storage();
		let state = reducer.reducer_state().await;
		let log = Log::new(co.as_bytes().to_vec(), self.inner.identity_resolver().await?, state.heads());

		// stream
		let stream = log.into_stream(&storage).map_err(|e| e.into());

		// result
		Ok((storage, stream))
	}

	/// Test if `co` is a shared CO.
	pub async fn is_shared(&self, co: &CoId) -> bool {
		self.inner.is_shared(co).await
	}

	/// Identities.
	///
	/// Todo: Identity Permissions?
	pub async fn identity_resolver(&self) -> Result<IdentityResolverBox, anyhow::Error> {
		self.inner.identity_resolver().await
	}

	/// Access Private Identities.
	///
	/// Todo: Identity Permissions?
	pub async fn private_identity_resolver(&self) -> Result<PrivateIdentityResolverBox, anyhow::Error> {
		self.inner.private_identity_resolver().await
	}

	/// Get unsiged local device identity.
	pub fn local_identity(&self) -> LocalIdentity {
		LocalIdentity::device()
	}

	/// Network.
	pub async fn network(&self) -> Option<(CoNetworkTaskSpawner, ActorHandle<ConnectionMessage>)> {
		self.inner.network.read().unwrap().clone()
	}

	/// Network Spawner.
	pub async fn network_tasks(&self) -> Option<CoNetworkTaskSpawner> {
		self.inner.network.read().unwrap().as_ref().map(|(v, _)| v).cloned()
	}

	/// Network Connections.
	pub async fn network_connections(&self) -> Option<ActorHandle<ConnectionMessage>> {
		self.inner.network.read().unwrap().as_ref().map(|(_, v)| v).cloned()
	}

	/// Tasks.
	pub fn tasks(&self) -> TaskSpawner {
		self.inner.tasks.clone()
	}

	/// Application identifier.
	pub fn identifier(&self) -> &str {
		&self.inner.settings.identifier
	}

	/// Application settings.
	pub fn settings(&self) -> &ApplicationSettings {
		&self.inner.settings
	}

	/// Force refresh co instance.
	pub async fn refresh(&self, co: CoReducer) -> Result<(), anyhow::Error> {
		let parent = match co.parent_id() {
			Some(parent) => self.try_co_reducer(&parent).await?,
			None => co.clone(),
		};
		co.context.refresh(parent, co.clone()).await?;
		Ok(())
	}
}
#[async_trait]
impl CoReducerFactory for CoContext {
	#[tracing::instrument(level = tracing::Level::TRACE, skip(self), fields(application = self.inner.settings.identifier))]
	async fn co_reducer(&self, co: &CoId) -> Result<Option<CoReducer>, anyhow::Error> {
		match self.try_co_reducer(co).await {
			Ok(r) => Ok(Some(r)),
			Err(CoReducerFactoryError::CoNotFound(_)) => Ok(None),
			Err(err) => Err(err.into()),
		}
	}

	#[tracing::instrument(skip(self), fields(application = self.inner.settings.identifier))]
	async fn try_co_reducer(&self, co: &CoId) -> Result<CoReducer, CoReducerFactoryError> {
		self.inner.reducers.clone().reducer(co.clone()).await
	}
}
impl Debug for CoContext {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("CoContext")
			.field("application", &self.inner.settings.identifier)
			.finish()
	}
}

#[derive(Clone)]
pub(crate) struct CoContextInner {
	settings: ApplicationSettings,

	shutdown: CancellationToken,
	tasks: TaskSpawner,

	local_identity: LocalIdentity,

	network: Arc<RwLock<Option<(CoNetworkTaskSpawner, ActorHandle<ConnectionMessage>)>>>,

	_storage: CoStorage,

	/// Used to track all new blocks until we store the LocalCo again.
	#[cfg(feature = "pinning")]
	storage_created: ChangeBlockStorage<CoStorage>,

	runtime: Runtime,
	reactive_context: ActorHandle<ApplicationMessage>,

	reducers: ReducersControl,
}
impl CoContextInner {
	pub(crate) fn new(
		settings: ApplicationSettings,
		shutdown: CancellationToken,
		tasks: TaskSpawner,
		local_identity: LocalIdentity,
		network: Option<(CoNetworkTaskSpawner, ActorHandle<ConnectionMessage>)>,
		storage: CoStorage,
		_tmp_storage: CoStorage,
		runtime: Runtime,
		reactive_context: ActorHandle<ApplicationMessage>,
		reducers: ReducersControl,
	) -> Self {
		Self {
			#[cfg(feature = "pinning")]
			storage_created: ChangeBlockStorage::new(storage.clone()),
			settings,
			shutdown,
			tasks,
			local_identity,
			network: Arc::new(RwLock::new(network)),
			_storage: storage,
			runtime,
			reactive_context,
			reducers,
		}
	}

	pub fn application(&self) -> ActorHandle<ApplicationMessage> {
		self.reactive_context.clone()
	}

	/// Shutdown token.
	pub fn shutdown(&self) -> CancellationToken {
		self.shutdown.clone()
	}

	/// Test if `co` is a shared CO.
	pub async fn is_shared(&self, co: &CoId) -> bool {
		// currently on the local co is not shared
		// the call is async to be future proof when we may need to check some state
		co.as_str() != CO_ID_LOCAL
	}

	/// Identities.
	///
	/// Todo: Identity Permissions?
	pub async fn identity_resolver(&self) -> Result<IdentityResolverBox, anyhow::Error> {
		Ok(create_identity_resolver())
	}

	/// Access Private Identities.
	///
	/// Todo: Identity Permissions?
	pub async fn private_identity_resolver(&self) -> Result<PrivateIdentityResolverBox, anyhow::Error> {
		let local = self.reducers.clone().reducer(CoId::from(CO_ID_LOCAL)).await?;
		create_private_identity_resolver(local).await
	}

	/// Get the root storage.
	/// The returned storage tracks changes which will be flushed when the local co is written.
	pub fn storage(&self) -> CoStorage {
		#[cfg(feature = "pinning")]
		return CoStorage::new(self.storage_created.clone());
		#[cfg(not(feature = "pinning"))]
		return self._storage.clone();
	}

	pub fn runtime(&self) -> Runtime {
		self.runtime.clone()
	}

	pub fn reducers_control(&self) -> ReducersControl {
		self.reducers.clone()
	}

	/// Clone with network.
	pub async fn set_network(
		&self,
		network: Option<(CoNetworkTaskSpawner, ActorHandle<ConnectionMessage>)>,
	) -> Result<(), anyhow::Error> {
		// assign
		*self.network.write().unwrap() = network;

		// clear reducers
		self.reducers.clone().clear().await?;

		// result
		Ok(())
	}

	/// Creates a CoReducer instance of the Local CO.
	#[tracing::instrument(level = tracing::Level::TRACE, skip(self))]
	pub(crate) async fn create_local_co_instance(&self, initialize: bool) -> Result<CoReducer, anyhow::Error> {
		let core_resolver = |_reducer_context| {
			let local_id = CoId::new(CO_ID_LOCAL);
			let core_resolver = CoCoreResolver::default();
			#[cfg(feature = "pinning")]
			let core_resolver = ChangeCoreResolver::new(core_resolver, self.storage_created.clone());
			#[cfg(feature = "pinning")]
			let core_resolver = ReferenceCoreResolver::new(
				core_resolver,
				Some(CoPinningKey::State.to_string(&local_id)),
				_reducer_context,
			);
			let core_resolver = ReactiveCoreResolver::new(core_resolver, local_id, self.reactive_context.clone());
			let core_resolver = LogCoreResolver::new(core_resolver);
			core_resolver
		};
		let local_co = LocalCoBuilder::new(self.settings.clone(), self.local_identity.clone(), initialize);
		let local_co_reducer = local_co
			.build(
				self.storage().clone_with_settings(BlockStorageSettings::new().with_detached()),
				self.runtime.clone(),
				self.shutdown.child_token(),
				self.tasks.clone(),
				core_resolver,
			)
			.await?;
		Ok(local_co_reducer)
	}

	/// Creates the Core Resolver for a shared CO.
	pub(crate) fn create_co_core_resolver(&self, id: CoId) -> DynamicCoreResolver<CoStorage> {
		let core_resolver = CoCoreResolver::default();
		let core_resolver =
			ReactiveCoreResolver::<CoStorage, CoCoreResolver>::new(core_resolver, id, self.reactive_context.clone());
		let core_resolver = LogCoreResolver::new(core_resolver);
		let core_resolver = DynamicCoreResolver::new(core_resolver);
		core_resolver
	}

	/// Creates a CoReducer instance for a CO.
	pub(crate) async fn create_co_instance_membership<I>(
		&self,
		parent: CoReducer,
		membership: Membership,
		identity: I,
		storage: ReducerStorage,
		initialize: bool,
		network: bool,
	) -> Result<CoReducer, anyhow::Error>
	where
		I: PrivateIdentity + Debug + Send + Sync + Clone + 'static,
	{
		// resolver
		let core_resolver = self.create_co_core_resolver(membership.id.clone());

		// network
		let network = if network { self.network.read().unwrap().clone() } else { None };

		// reducer
		let reducer = SharedCoBuilder::new(parent, membership)
			.with_membership_core_name(CO_CORE_NAME_MEMBERSHIP.to_owned())
			.with_keystore_core_name(CO_CORE_NAME_KEYSTORE.to_owned())
			.with_storage_core_name(CO_CORE_NAME_STORAGE.to_owned())
			.with_network(network)
			.with_initialize(initialize)
			.build(self.tasks.clone(), storage, self.runtime.clone(), identity, core_resolver)
			.await?;

		// result
		Ok(reducer)
	}

	/// Creates a CoReducer instance a CO which we have a membership for.
	pub(crate) async fn create_co_instance(
		&self,
		parent: CoReducer,
		co: &CoId,
		storage: ReducerStorage,
		initialize: bool,
		identity: Option<Did>,
	) -> Result<Option<CoReducer>, anyhow::Error> {
		// find first active membership
		let membership = shared_membership(&parent, co, identity.as_ref()).await?;
		let membership = match membership {
			Some(m) => m,
			None => return Ok(None),
		};

		// resolve identity
		let identity = create_private_identity_resolver(parent.clone())
			.await?
			.resolve_private(&membership.did)
			.await?;

		// instance
		Ok(Some(
			self.create_co_instance_membership(parent, membership, identity, storage, initialize, true)
				.await?,
		))
	}
}
impl From<CoContextInner> for CoContext {
	fn from(val: CoContextInner) -> Self {
		CoContext { inner: Arc::new(val) }
	}
}
