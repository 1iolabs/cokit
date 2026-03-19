// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{
	application::{
		application::ApplicationSettings,
		identity::{create_identity_resolver, create_private_identity_resolver},
		local::LocalCoContext,
		shared::{SharedCoBuilder, SharedCoCreator},
	},
	library::{
		builtin_cores::builtin_cores,
		contact_handler::DynamicContactHandler,
		shared_membership::{shared_membership_active, wait_shared_membership_active},
		wait_response::request_response,
	},
	reducer::core_resolver::{dynamic::DynamicCoreResolver, guard::CoGuardResolver, log::LogCoreResolver},
	services::{
		application::{ApplicationMessage, ContactAction},
		reducers::{ReducerOptions, ReducerStorage, ReducersControl},
	},
	state,
	types::co_reducer_factory::CoReducerFactoryError,
	Action, CoCoreResolver, CoOptions, CoReducer, CoReducerFactory, CoStorage, Cores, CreateCo, DynamicCoAccessPolicy,
	DynamicCoUuid, DynamicLocalSecret, Guards, LocalCoBuilder, Runtime, Storage, TaskSpawner, CO_CORE_NAME_KEYSTORE,
	CO_CORE_NAME_MEMBERSHIP, CO_ID_LOCAL,
};
use async_trait::async_trait;
use cid::Cid;
use co_actor::{time, ActorHandle};
use co_core_membership::Membership;
use co_identity::{
	IdentityResolverBox, LocalIdentity, PrivateIdentity, PrivateIdentityResolver, PrivateIdentityResolverBox,
};
use co_log::{EntryBlock, Log};
#[cfg(feature = "network")]
use co_network::{connections::ConnectionMessage, HeadsApi, NetworkApi};
use co_primitives::{
	BlockLinks, BlockStorageCloneSettings, CloneWithBlockStorageSettings, CoId, Did, DynamicCoDate, IgnoreFilter,
	Network,
};
use futures::{FutureExt, Stream, TryStreamExt};
use std::{
	collections::{BTreeMap, BTreeSet},
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
	//#[deprecated(note = "Use co_sdk::state::heads instead")]
	pub async fn entries(
		&self,
		co: impl AsRef<CoId>,
	) -> Result<(CoStorage, impl Stream<Item = Result<EntryBlock, anyhow::Error>>), anyhow::Error> {
		// log
		let reducer = self.try_co_reducer(co.as_ref()).await?;
		let storage = reducer.storage();
		let state = reducer.reducer_state().await;

		// stream
		let stream = state::heads_stream(storage.clone(), co.as_ref(), state.heads());

		// result
		Ok((storage, stream))
	}

	/// Get a stream to the log entries.
	/// Starting at `heads` (reverse chronological).
	#[deprecated(note = "Use co_sdk::state::heads instead")]
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
		let stream = log.into_stream(storage).map_err(|e| e.into());

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

	/// Get unsigned local device identity.
	pub fn local_identity(&self) -> LocalIdentity {
		LocalIdentity::device()
	}

	/// Network.
	#[cfg(feature = "network")]
	pub async fn network(&self) -> Option<NetworkApi> {
		self.inner.network.read().unwrap().clone()
	}

	/// Network Connections.
	#[cfg(feature = "network")]
	pub async fn network_connections(&self) -> Option<ActorHandle<ConnectionMessage>> {
		self.inner.network.read().unwrap().as_ref().map(|api| api.connections().clone())
	}

	/// Network Heads.
	#[cfg(feature = "network")]
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

	/// Builtin cores.
	pub fn cores(&self) -> &Cores {
		&self.inner.cores
	}

	/// Block links reader.
	pub fn block_links(&self, exclude_builtin: bool) -> &BlockLinks {
		if exclude_builtin {
			&self.inner.block_links_builtin
		} else {
			&self.inner.block_links
		}
	}

	/// CO access policy for non-participants.
	pub fn access_policy(&self) -> Option<&DynamicCoAccessPolicy> {
		self.inner.access_policy.as_ref()
	}

	/// Contact handler for incoming contact requests.
	pub fn contact_handler(&self) -> Option<&DynamicContactHandler> {
		self.inner.contact_handler.as_ref()
	}

	/// Send a contact request to a DID.
	///
	/// # Return
	/// This method returns whether the contact request could be send to to recipient.
	/// Note that the actual contact can decide if and when he want to connect back.
	pub async fn contact(
		&self,
		from: Did,
		to: Did,
		subject: Option<String>,
		headers: BTreeMap<String, String>,
		networks: impl IntoIterator<Item = Network>,
	) -> Result<(), anyhow::Error> {
		let contact =
			ContactAction { from, to, sub: subject, networks: networks.into_iter().collect(), fields: headers };

		let result: Result<(), crate::ActionError> =
			request_response(self.inner.application(), Action::Contact(contact.clone()), move |action| match action {
				Action::ContactSent(sent, result) if *sent == contact => Some(result.clone()),
				_ => None,
			})
			.await?;

		result.map_err(|err| err.into())
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

	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), skip(self), fields(application = self.inner.settings.identifier))]
	async fn try_co_reducer_with_options(
		&self,
		co: &CoId,
		options: CoOptions,
	) -> Result<CoReducer, CoReducerFactoryError> {
		self.inner
			.reducers
			.clone()
			.reducer(co.clone(), ReducerOptions::default().with_co_options(options))
			.await
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

	#[cfg(feature = "network")]
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
	guards: Guards,
	local_secret: Option<DynamicLocalSecret>,
	access_policy: Option<DynamicCoAccessPolicy>,
	contact_handler: Option<DynamicContactHandler>,
}
impl CoContextInner {
	#[allow(clippy::too_many_arguments)]
	pub(crate) fn new(
		settings: ApplicationSettings,
		shutdown: CancellationToken,
		tasks: TaskSpawner,
		local_identity: LocalIdentity,
		#[cfg(feature = "network")] network: Option<NetworkApi>,
		storage: Storage,
		runtime: Runtime,
		reactive_context: ActorHandle<ApplicationMessage>,
		reducers: ReducersControl,
		date: DynamicCoDate,
		uuid: DynamicCoUuid,
		cores: Cores,
		guards: Guards,
		local_secret: Option<DynamicLocalSecret>,
		access_policy: Option<DynamicCoAccessPolicy>,
		contact_handler: Option<DynamicContactHandler>,
	) -> Self {
		let block_links = BlockLinks::default();
		let block_links_builtin = block_links.clone().with_filter(IgnoreFilter::new(builtin_cores()));
		Self {
			settings,
			shutdown,
			tasks,
			local_identity,
			#[cfg(feature = "network")]
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
			guards,
			local_secret,
			access_policy,
			contact_handler,
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
	#[cfg(feature = "network")]
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
			.with_local_secret(self.local_secret.clone())
			.with_verify_links(
				self.settings
					.feature_co_storage_verify_links()
					.then(|| self.block_links_builtin.clone()),
			);
		let local_co_reducer = local_co
			.build(
				LocalCoContext {
					storage: self
						.storage()
						.clone_with_settings(BlockStorageCloneSettings::new().with_detached()),
					runtime: self.runtime.clone(),
					shutdown: self.shutdown.child_token(),
					tasks: self.tasks.clone(),
					core_resolver: self.create_local_core_resolver(CoId::new(CO_ID_LOCAL)),
					date: self.date.clone(),
					application_handle: self.application(),
					#[cfg(feature = "pinning")]
					pinning: self.create_pinning_context(),
				},
				&self.cores,
			)
			.boxed()
			.await?;
		Ok(local_co_reducer)
	}

	/// Creates the Core Resolver for the local CO.
	fn create_local_core_resolver(&self, id: CoId) -> DynamicCoreResolver<CoStorage> {
		let core_resolver = CoCoreResolver::new(&self.cores);
		let core_resolver = LogCoreResolver::new(core_resolver, id, self.date.clone());
		DynamicCoreResolver::new(core_resolver)
	}

	/// Creates the Core Resolver for a shared CO.
	pub(crate) fn create_shared_core_resolver(&self, id: CoId) -> DynamicCoreResolver<CoStorage> {
		let core_resolver = CoCoreResolver::new(&self.cores);
		let core_resolver =
			CoGuardResolver::new(core_resolver, &self.guards).with_ignore_mode(self.settings.feature_co_guard_ignore());
		let core_resolver = LogCoreResolver::new(core_resolver, id, self.date.clone());
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
		options: CoOptions,
	) -> Result<Option<CoReducer>, anyhow::Error> {
		// find first active membership
		let membership = if options.wait {
			if let Some(timeout) = options.wait_timeout {
				time::timeout(timeout, wait_shared_membership_active(&parent, co, identity.as_ref())).await??
			} else {
				wait_shared_membership_active(&parent, co, identity.as_ref()).await?
			}
		} else {
			shared_membership_active(&parent, co, identity.as_ref()).await?
		};
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

	/// Create a new shared CO.
	pub async fn create_co<I>(&self, parent: CoReducer, creator: I, create: CreateCo) -> Result<CoId, anyhow::Error>
	where
		I: PrivateIdentity + Clone + Debug + Send + Sync + 'static,
	{
		// create
		let co = SharedCoCreator::new(parent, create)
			.with_membership_core_name(CO_CORE_NAME_MEMBERSHIP.to_string())
			.with_keystore_core_name(CO_CORE_NAME_KEYSTORE.to_string())
			.create(
				self.storage(),
				self.runtime.clone(),
				&self.cores,
				creator,
				self.date.clone(),
				self.uuid.clone(),
				#[cfg(feature = "pinning")]
				self.create_pinning_context(),
				#[cfg(feature = "pinning")]
				Default::default(),
			)
			.await?;

		// result
		Ok(co)
	}
}
impl From<CoContextInner> for CoContext {
	fn from(val: CoContextInner) -> Self {
		CoContext { inner: Arc::new(val) }
	}
}
