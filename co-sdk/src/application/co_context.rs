use crate::{
	application::{
		application::ApplicationSettings,
		identity::{create_identity_resolver, create_private_identity_resolver},
		shared::SharedCoBuilder,
	},
	library::{builtin_cores::builtin_cores, shared_membership::shared_membership_active},
	reducer::core_resolver::{dynamic::DynamicCoreResolver, guard::CoGuardResolver, log::LogCoreResolver},
	services::{
		application::ApplicationMessage,
		reducers::{ReducerStorage, ReducersControl},
	},
	types::co_reducer_factory::CoReducerFactoryError,
	CoCoreResolver, CoReducer, CoReducerFactory, CoStorage, Cores, DynamicCoDate, DynamicCoUuid, LocalCoBuilder,
	Runtime, Storage, TaskSpawner, CO_CORE_NAME_KEYSTORE, CO_CORE_NAME_MEMBERSHIP, CO_ID_LOCAL,
};
use async_trait::async_trait;
use cid::Cid;
use co_actor::ActorHandle;
use co_core_membership::Membership;
use co_identity::{
	IdentityResolverBox, LocalIdentity, PrivateIdentity, PrivateIdentityResolver, PrivateIdentityResolverBox,
};
use co_log::{EntryBlock, Log};
use co_network::{connections::ConnectionMessage, HeadsApi, NetworkApi};
use co_primitives::{BlockLinks, BlockStorageSettings, CloneWithBlockStorageSettings, CoId, Did, IgnoreFilter};
use futures::{Stream, TryStreamExt};
use std::{
	collections::BTreeSet,
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
	#[tracing::instrument(level = tracing::Level::TRACE, skip(self), fields(application = self.inner.settings.identifier))]
	pub async fn local_co_reducer(&self) -> Result<CoReducer, anyhow::Error> {
		Ok(self
			.inner
			.reducers
			.clone()
			.reducer(CoId::from(CO_ID_LOCAL), Default::default())
			.await?)
	}

	/// Get a stream to the log entries.
	/// Starting at the latest (reverse chronological).
	/// The stream is read with snapshot isolation (not watching changes).
	pub async fn entries(
		&self,
		co: impl AsRef<CoId>,
	) -> Result<(CoStorage, impl Stream<Item = Result<EntryBlock, anyhow::Error>>), anyhow::Error> {
		// log
		let reducer = self.try_co_reducer(co.as_ref()).await?;
		let storage = reducer.storage();
		let state = reducer.reducer_state().await;

		// stream
		let stream = self.entries_from_heads(co, storage.clone(), state.1).await?;

		// result
		Ok((storage, stream))
	}

	/// Get a stream to the log entries.
	/// Starting at `heads` (reverse chronological).
	pub async fn entries_from_heads(
		&self,
		co: impl AsRef<CoId>,
		storage: CoStorage,
		heads: BTreeSet<Cid>,
	) -> Result<impl Stream<Item = Result<EntryBlock, anyhow::Error>>, anyhow::Error> {
		let co = co.as_ref();

		// log
		let log = Log::new_readonly(co.as_bytes().to_vec(), heads);

		// stream
		let stream = log.into_stream(&storage).map_err(|e| e.into());

		// result
		Ok(stream)
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
	pub async fn network(&self) -> Option<NetworkApi> {
		self.inner.network.read().unwrap().clone()
	}

	/// Network Connections.
	pub async fn network_connections(&self) -> Option<ActorHandle<ConnectionMessage>> {
		self.inner.network.read().unwrap().as_ref().map(|api| api.connections().clone())
	}

	/// Network Heads.
	pub async fn network_heads(&self) -> Option<HeadsApi> {
		self.inner.network.read().unwrap().as_ref().map(|api| api.heads().clone())
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

	/// Date Source.
	pub fn date(&self) -> &DynamicCoDate {
		&self.inner.date
	}

	/// UUID Source.
	pub fn uuid(&self) -> &DynamicCoUuid {
		&self.inner.uuid
	}

	/// Block links reader.
	pub fn block_links(&self, exclude_builtin: bool) -> &BlockLinks {
		if exclude_builtin {
			&self.inner.block_links_builtin
		} else {
			&self.inner.block_links
		}
	}

	/// Force refresh co instance.
	pub async fn refresh(&self, co: CoReducer) -> Result<(), anyhow::Error> {
		let parent = match co.parent_id() {
			Some(parent) => self.try_co_reducer(parent).await?,
			None => co.clone(),
		};
		co.context.refresh(parent, co.clone()).await?;
		Ok(())
	}
}
#[async_trait]
impl CoReducerFactory for CoContext {
	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), skip(self), fields(application = self.inner.settings.identifier))]
	async fn co_reducer(&self, co: &CoId) -> Result<Option<CoReducer>, anyhow::Error> {
		match self.try_co_reducer(co).await {
			Ok(r) => Ok(Some(r)),
			Err(CoReducerFactoryError::CoNotFound(_, _)) => Ok(None),
			Err(err) => Err(err.into()),
		}
	}

	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), skip(self), fields(application = self.inner.settings.identifier))]
	async fn try_co_reducer(&self, co: &CoId) -> Result<CoReducer, CoReducerFactoryError> {
		self.inner.reducers.clone().reducer(co.clone(), Default::default()).await
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

	network: Arc<RwLock<Option<NetworkApi>>>,

	storage: Storage,

	runtime: Runtime,
	reactive_context: ActorHandle<ApplicationMessage>,

	reducers: ReducersControl,
	date: DynamicCoDate,
	uuid: DynamicCoUuid,
	block_links: BlockLinks,
	block_links_builtin: BlockLinks,
	cores: Cores,
}
impl CoContextInner {
	pub(crate) fn new(
		settings: ApplicationSettings,
		shutdown: CancellationToken,
		tasks: TaskSpawner,
		local_identity: LocalIdentity,
		network: Option<NetworkApi>,
		storage: Storage,
		runtime: Runtime,
		reactive_context: ActorHandle<ApplicationMessage>,
		reducers: ReducersControl,
		date: DynamicCoDate,
		uuid: DynamicCoUuid,
		cores: Cores,
	) -> Self {
		let block_links = BlockLinks::default();
		let block_links_builtin = block_links.clone().with_filter(IgnoreFilter::new(builtin_cores()));
		Self {
			settings,
			shutdown,
			tasks,
			local_identity,
			network: Arc::new(RwLock::new(network)),
			storage,
			runtime,
			reactive_context,
			reducers,
			date,
			uuid,
			block_links,
			block_links_builtin,
			cores,
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
		let local = self
			.reducers
			.clone()
			.reducer(CoId::from(CO_ID_LOCAL), Default::default())
			.await?;
		create_private_identity_resolver(local).await
	}

	/// Get the application storage.
	pub fn application_storage(&self) -> &Storage {
		&self.storage
	}

	/// Get the root storage.
	pub fn storage(&self) -> CoStorage {
		self.storage.storage()
	}

	pub fn runtime(&self) -> Runtime {
		self.runtime.clone()
	}

	pub fn reducers_control(&self) -> ReducersControl {
		self.reducers.clone()
	}

	/// Clone with network.
	pub async fn set_network(&self, network: Option<NetworkApi>) -> Result<(), anyhow::Error> {
		// assign
		*self.network.write().unwrap() = network;

		// result
		Ok(())
	}

	/// Creates a CoReducer instance of the Local CO.
	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), skip(self))]
	pub(crate) async fn create_local_co_instance(&self, initialize: bool) -> Result<CoReducer, anyhow::Error> {
		let local_co = LocalCoBuilder::new(self.settings.clone(), self.local_identity.clone(), initialize)
			.with_verify_links(
				self.settings
					.feature_co_storage_verify_links()
					.then(|| self.block_links_builtin.clone()),
			);
		let local_co_reducer = local_co
			.build(
				self.storage().clone_with_settings(BlockStorageSettings::new().with_detached()),
				self.runtime.clone(),
				self.shutdown.child_token(),
				self.tasks.clone(),
				self.create_local_core_resolver(CoId::new(CO_ID_LOCAL)),
				self.date.clone(),
				self.application(),
				#[cfg(feature = "pinning")]
				self.create_pinning_context(),
			)
			.await?;
		Ok(local_co_reducer)
	}

	/// Creates the Core Resolver for the local CO.
	fn create_local_core_resolver(&self, id: CoId) -> DynamicCoreResolver<CoStorage> {
		let core_resolver = CoCoreResolver::new(&self.cores);
		let core_resolver = LogCoreResolver::new(core_resolver, id);
		DynamicCoreResolver::new(core_resolver)
	}

	/// Creates the Core Resolver for a shared CO.
	pub(crate) fn create_shared_core_resolver(&self, id: CoId) -> DynamicCoreResolver<CoStorage> {
		let core_resolver = CoCoreResolver::new(&self.cores);
		let core_resolver = CoGuardResolver::new(core_resolver);
		let core_resolver = LogCoreResolver::new(core_resolver, id);
		DynamicCoreResolver::new(core_resolver)
	}

	/// Creates a CoReducer instance for a CO.
	pub(crate) async fn create_co_instance_membership<I>(
		&self,
		parent: CoReducer,
		membership: Membership,
		identity: I,
		storage: ReducerStorage,
		initialize: bool,
	) -> Result<CoReducer, anyhow::Error>
	where
		I: PrivateIdentity + Debug + Send + Sync + Clone + 'static,
	{
		// resolver
		let core_resolver = self.create_shared_core_resolver(membership.id.clone());

		// reducer
		let reducer = SharedCoBuilder::new(parent, membership)
			.with_membership_core_name(CO_CORE_NAME_MEMBERSHIP.to_string())
			.with_keystore_core_name(CO_CORE_NAME_KEYSTORE.to_string())
			.with_verify_links(
				self.settings
					.feature_co_storage_verify_links()
					.then(|| self.block_links_builtin.clone()),
			)
			.with_initialize(initialize)
			.build(
				self.tasks.clone(),
				storage,
				self.runtime.clone(),
				identity,
				core_resolver,
				self.date.clone(),
				self.application(),
				#[cfg(feature = "pinning")]
				self.create_pinning_context(),
				#[cfg(feature = "pinning")]
				self.settings.setting_co_default_max_state(),
			)
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
		let membership = shared_membership_active(&parent, co, identity.as_ref()).await?;
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
			self.create_co_instance_membership(parent, membership, identity, storage, initialize)
				.await?,
		))
	}

	#[cfg(feature = "pinning")]
	pub(crate) fn create_pinning_context(&self) -> crate::library::storage_pinning::StoragePinningContext {
		crate::library::storage_pinning::StoragePinningContext {
			identity: self.local_identity.clone().boxed(),
			storage: self.storage.clone(),
			runtime: self.runtime(),
			date: self.date.clone(),
			tasks: self.tasks.clone(),
			block_links: self.block_links.clone(),
			free: self.settings.feature_co_storage_free(),
			verify_links: self
				.settings
				.feature_co_storage_verify_links()
				.then(|| self.block_links_builtin.clone()),
		}
	}
}
impl From<CoContextInner> for CoContext {
	fn from(val: CoContextInner) -> Self {
		CoContext { inner: Arc::new(val) }
	}
}
