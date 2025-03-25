use super::{
	application::ApplicationSettings,
	identity::{create_identity_resolver, create_private_identity_resolver},
	shared::SharedCoBuilder,
};
use crate::{
	library::find_membership::memberships,
	reducer::core_resolver::{
		change::ChangeCoreResolver,
		dynamic::DynamicCoreResolver,
		epic::ReactiveCoreResolver,
		log::LogCoreResolver,
		membership::{MembershipCoreResolver, MembershipInstanceRegistry},
		reference::ReferenceCoreResolver,
	},
	services::{application::ApplicationMessage, connections::ConnectionMessage, network::CoNetworkTaskSpawner},
	types::{co_reducer::CoReducerContext, co_reducer_factory::CoReducerFactoryError},
	CoCoreResolver, CoReducer, CoReducerFactory, CoStorage, Cores, LocalCoBuilder, Runtime, TaskSpawner,
	CO_CORE_NAME_KEYSTORE, CO_CORE_NAME_MEMBERSHIP, CO_CORE_NAME_STORAGE, CO_ID_LOCAL,
};
use anyhow::anyhow;
use async_trait::async_trait;
use co_actor::{Actor, ActorError, ActorHandle, Response};
use co_core_membership::Membership;
use co_identity::{
	IdentityResolverBox, LocalIdentity, PrivateIdentity, PrivateIdentityResolver, PrivateIdentityResolverBox,
};
use co_log::EntryBlock;
use co_primitives::{BlockStorageSettings, CloneWithBlockStorageSettings, CoId, Did, Tags};
use co_storage::{BlockStorage, ChangeBlockStorage, EncryptedBlockStorage, EncryptionReferenceMode, StorageError};
use futures::{Stream, TryStreamExt};
use std::{
	collections::{BTreeMap, VecDeque},
	fmt::Debug,
	sync::Arc,
};
use tokio::sync::RwLock;
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
			let storage = self.inner.reducers.clone().storage(co.clone()).await?;
			self.inner
				.create_co_instance(local, co, storage, initialized, None)
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
		self.inner.network.read().await.clone()
	}

	/// Network Spawner.
	pub async fn network_tasks(&self) -> Option<CoNetworkTaskSpawner> {
		self.inner.network.read().await.as_ref().map(|(v, _)| v).cloned()
	}

	/// Network Connections.
	pub async fn network_connections(&self) -> Option<ActorHandle<ConnectionMessage>> {
		self.inner.network.read().await.as_ref().map(|(_, v)| v).cloned()
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
}
#[async_trait]
impl CoReducerFactory for CoContext {
	#[tracing::instrument(skip(self), fields(application = self.inner.settings.identifier))]
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

#[derive(Debug)]
pub enum ReducerRequest {
	/// Request CO storage instance (without networking).
	Storage(CoId, Response<Result<ReducerStorage, CoReducerFactoryError>>),
	/// Request CO reducer instance by creating it if not created yet.
	Request(CoId, Response<Result<CoReducer, CoReducerFactoryError>>),
	/// Create reducer instance.
	Create(CoId, Result<CoReducer, CoReducerFactoryError>),
	/// Create shared storage instance.
	CreateStorage(CoId, Result<ReducerStorage, CoReducerFactoryError>),
	/// Clear all reducer instances.
	Clear(Response<Result<(), CoReducerFactoryError>>),
	/// Clear a specific reducer instance.
	ClearOne(CoId, Response<Result<(), CoReducerFactoryError>>),
}

#[derive(Clone)]
pub struct ReducersControl {
	pub(crate) handle: ActorHandle<ReducerRequest>,
}
impl ReducersControl {
	pub async fn storage(&self, co: CoId) -> Result<ReducerStorage, CoReducerFactoryError> {
		// tracing::trace!(?co, err = ?anyhow::anyhow!("test"), "co-reducer-request");
		Ok(self.handle.request(|response| ReducerRequest::Storage(co, response)).await??)
	}

	pub async fn reducer(&self, co: CoId) -> Result<CoReducer, CoReducerFactoryError> {
		// tracing::trace!(?co, err = ?anyhow::anyhow!("test"), "co-reducer-request");
		Ok(self.handle.request(|response| ReducerRequest::Request(co, response)).await??)
	}

	pub async fn create(&self, co: CoId, reducer: Result<CoReducer, CoReducerFactoryError>) {
		self.handle.dispatch(ReducerRequest::Create(co, reducer)).ok();
	}

	pub async fn create_storage(&self, co: CoId, storage: Result<ReducerStorage, CoReducerFactoryError>) {
		self.handle.dispatch(ReducerRequest::CreateStorage(co, storage)).ok();
	}

	pub async fn clear(&self) -> Result<(), CoReducerFactoryError> {
		Ok(self.handle.request(|response| ReducerRequest::Clear(response)).await??)
	}

	pub async fn clear_one(&self, co: CoId) -> Result<(), CoReducerFactoryError> {
		Ok(self.handle.request(|response| ReducerRequest::ClearOne(co, response)).await??)
	}
}
impl From<ActorHandle<ReducerRequest>> for ReducersControl {
	fn from(value: ActorHandle<ReducerRequest>) -> Self {
		Self { handle: value }
	}
}

pub struct Reducers {
	context: CoContext,
	reducers: BTreeMap<CoId, CoReducer>,
	storages: BTreeMap<CoId, ReducerStorage>,
	pending_requests: VecDeque<ReducerRequest>,
}
impl Reducers {
	async fn local(&mut self) -> Result<CoReducer, CoReducerFactoryError> {
		let local_id = CoId::from(CO_ID_LOCAL);
		let local = if let Some(local) = self.reducers.get(&local_id) {
			local.clone()
		} else {
			let local = self.context.inner.create_local_co_instance(true).await?;
			self.reducers.insert(local.id().clone(), local.clone());
			local
		};
		Ok(local)
	}

	fn pending_request_count(&self, co: &CoId) -> usize {
		self.pending_requests.iter().fold(0, |a, b| match b {
			ReducerRequest::Request(id, _) if id == co => a + 1,
			_ => a,
		})
	}

	fn pending_storage_count(&self, co: &CoId) -> usize {
		self.pending_requests.iter().fold(0, |a, b| match b {
			ReducerRequest::Storage(id, _) if id == co => a + 1,
			_ => a,
		})
	}
}

pub struct ReducersActor {}
impl ReducersActor {
	pub fn new() -> Self {
		Self {}
	}
}
#[async_trait]
impl Actor for ReducersActor {
	type Message = ReducerRequest;
	type State = Reducers;
	type Initialize = CoContext;

	async fn initialize(
		&self,
		_handle: &ActorHandle<Self::Message>,
		_tags: &Tags,
		initialize: Self::Initialize,
	) -> Result<Self::State, ActorError> {
		Ok(Reducers {
			context: initialize,
			reducers: Default::default(),
			storages: Default::default(),
			pending_requests: Default::default(),
		})
	}

	async fn handle(
		&self,
		handle: &ActorHandle<Self::Message>,
		message: Self::Message,
		state: &mut Self::State,
	) -> Result<(), ActorError> {
		match message {
			ReducerRequest::Storage(id, response) => {
				// local
				let local = match state.local().await {
					Ok(local) => local,
					Err(err) => {
						response
							.send(Err(CoReducerFactoryError::Create(CoId::from(CO_ID_LOCAL), err.into())))
							.ok();
						return Ok(());
					},
				};

				// get/create
				if let Some(storage) = state.storages.get(&id) {
					response.send(Ok(storage.clone())).ok();
				} else {
					state.pending_requests.push_back(ReducerRequest::Storage(id.clone(), response));
					if state.pending_storage_count(&id) == 1 {
						// create storage
						state.context.tasks().spawn({
							let control: ReducersControl = handle.clone().into();
							let context = state.context.clone();
							let parent = local.clone();
							async move {
								let result = ReducerStorage::from_id(
									context
										.inner
										.storage()
										.clone_with_settings(BlockStorageSettings::new().with_detached()),
									parent,
									id.clone(),
								)
								.await;
								control.create_storage(id, result).await;
							}
						});
					}
				}
			},
			ReducerRequest::Request(id, response) => {
				// local
				let local = match state.local().await {
					Ok(local) => local,
					Err(err) => {
						response
							.send(Err(CoReducerFactoryError::Create(CoId::from(CO_ID_LOCAL), err.into())))
							.ok();
						return Ok(());
					},
				};

				// get/create
				if let Some(reducer) = state.reducers.get(&id) {
					response.send(Ok(reducer.clone())).ok();
				} else {
					state.pending_requests.push_back(ReducerRequest::Request(id.clone(), response));
					if state.pending_request_count(&id) == 1 {
						// create shared co
						state.context.tasks().spawn({
							let control: ReducersControl = handle.clone().into();
							let context = state.context.clone();
							let parent = local.clone();
							async move {
								// get storage
								let result = match control.clone().storage(id.clone()).await {
									Ok(storage) => {
										// create reducer
										match context.inner.create_co_instance(parent, &id, storage, true, None).await {
											Ok(Some(reducer)) => Ok(reducer),
											Ok(None) => Err(CoReducerFactoryError::CoNotFound(id.clone())),
											Err(err) => Err(CoReducerFactoryError::Create(id.clone(), err)),
										}
									},
									Err(err) => Err(err),
								};

								// notify
								control.clone().create(id, result).await;
							}
						});
					}
				}
			},
			ReducerRequest::Clear(response) => {
				state.reducers.retain(|id, _| id.as_str() == CO_ID_LOCAL);
				response.send(Ok(())).ok();
			},
			ReducerRequest::ClearOne(id, response) => {
				state.reducers.retain(|retain_id, _| retain_id != &id);
				response.send(Ok(())).ok();
			},
			ReducerRequest::Create(id, result) => {
				// register
				match &result {
					Ok(reducer) => {
						state.reducers.insert(reducer.id().clone(), reducer.clone());
					},
					Err(err) => {
						tracing::error!(co = ?id, ?err, "co-reducer-failed");
					},
				}

				// respond pending
				let remove = state
					.pending_requests
					.iter()
					.enumerate()
					.filter_map(|(index, request)| match request {
						ReducerRequest::Request(request_id, _) if request_id == &id => Some(index),
						_ => None,
					})
					.rev()
					.collect::<Vec<_>>();
				for index in remove {
					if let Some(ReducerRequest::Request(_, response)) = state.pending_requests.remove(index) {
						response
							.send(match &result {
								Err(err) => Err(co_reducerfactory_error_clone(err)),
								Ok(reducer) => Ok(reducer.clone()),
							})
							.ok();
					}
				}
			},
			ReducerRequest::CreateStorage(id, result) => {
				// register
				match &result {
					Ok(storage) => {
						state.storages.insert(id.clone(), storage.clone());
					},
					Err(err) => {
						tracing::error!(co = ?id, ?err, "co-storage-failed");
					},
				}

				// respond pending
				let mut remove = state
					.pending_requests
					.iter()
					.enumerate()
					.filter_map(|(index, request)| match request {
						ReducerRequest::Storage(request_id, _) if request_id == &id => Some(index),
						_ => None,
					})
					.collect::<VecDeque<usize>>();
				while let Some(index) = remove.pop_back() {
					if let Some(ReducerRequest::Storage(_, response)) = state.pending_requests.remove(index) {
						// for the last element send the original result
						if remove.is_empty() {
							response.send(result).ok();
							break;
						} else {
							response
								.send(match &result {
									Err(err) => Err(co_reducerfactory_error_clone(err)),
									Ok(storage) => Ok(storage.clone()),
								})
								.ok();
						}
					}
				}
			},
		}
		return Ok(());
	}
}

fn co_reducerfactory_error_clone(err: &CoReducerFactoryError) -> CoReducerFactoryError {
	match err {
		CoReducerFactoryError::CoNotFound(id) => CoReducerFactoryError::CoNotFound(id.clone()),
		CoReducerFactoryError::Create(id, err) => {
			CoReducerFactoryError::Create(id.to_owned(), anyhow!(err.to_string()))
		},
		CoReducerFactoryError::Other(err) => CoReducerFactoryError::Other(anyhow!(err.to_string())),
		CoReducerFactoryError::Actor(err) => CoReducerFactoryError::Other(anyhow!(err.to_string())),
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
		// self.storage.clone()
		CoStorage::new(self.storage_created.clone())
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
		*self.network.write().await = network;

		// clear reducers
		self.reducers.clone().clear().await?;

		// result
		Ok(())
	}

	/// Creates a CoReducer instance of the Local CO.
	#[tracing::instrument(skip(self))]
	async fn create_local_co_instance(&self, initialize: bool) -> Result<CoReducer, anyhow::Error> {
		let core_resolver = |reducer_context| {
			let local_id = CoId::new(CO_ID_LOCAL);
			let core_resolver = CoCoreResolver::default();
			let core_resolver = ChangeCoreResolver::new(core_resolver, self.storage_created.clone());
			let core_resolver = ReferenceCoreResolver::new(
				core_resolver,
				Some(CoPinningKey::State.to_string(&local_id)),
				reducer_context,
			);
			let core_resolver = ReactiveCoreResolver::new(core_resolver, local_id, self.reactive_context.clone());
			let core_resolver = MembershipCoreResolver::new(
				self.tasks.clone(),
				core_resolver,
				CoContextMembershipInstanceRegistry { reducers: self.reducers.clone() },
				CO_CORE_NAME_MEMBERSHIP.to_owned(),
			);
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
		let network = if network { self.network.read().await.clone() } else { None };

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
	async fn create_co_instance(
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

/// Find shared membership.
async fn shared_membership(
	parent: &CoReducer,
	co: &CoId,
	identity: Option<&Did>,
) -> Result<Option<Membership>, anyhow::Error> {
	// find first active membership
	Ok(memberships(&parent, &co).await?.find(move |membership| match identity {
		Some(value) => value == &membership.did,
		None => true,
	}))
}

pub enum CoPinningKey {
	State,
	Log,
}
impl CoPinningKey {
	pub fn to_string(&self, co: &CoId) -> String {
		match self {
			CoPinningKey::State => format!("co.{}.state", co.as_str()),
			CoPinningKey::Log => format!("co.{}.log", co.as_str()),
		}
	}
}

/// Reducer storage implementations.
#[derive(Debug, Clone)]
pub enum ReducerStorage {
	Default(CoStorage),
	Encrypted(CoStorage, EncryptedBlockStorage<CoStorage>),
}
impl ReducerStorage {
	pub fn storage(&self) -> &CoStorage {
		match self {
			ReducerStorage::Default(storage) => storage,
			ReducerStorage::Encrypted(storage, _encrypted) => storage,
		}
	}

	pub fn encrypted_storage(&self) -> Option<&EncryptedBlockStorage<CoStorage>> {
		match self {
			ReducerStorage::Default(_) => None,
			ReducerStorage::Encrypted(_, encrypted) => Some(encrypted),
		}
	}

	async fn from_id(storage: CoStorage, parent: CoReducer, id: CoId) -> Result<ReducerStorage, CoReducerFactoryError> {
		let membership = shared_membership(&parent, &id, None)
			.await?
			.ok_or(CoReducerFactoryError::CoNotFound(id))?;
		Ok(Self::from_membership(&storage, &parent, membership)
			.await
			.map_err(|e| CoReducerFactoryError::Other(e.into()))?)
	}

	async fn from_membership(
		storage: &CoStorage,
		parent: &CoReducer,
		membership: Membership,
	) -> Result<ReducerStorage, StorageError> {
		let builder = SharedCoBuilder::new(parent.clone(), membership.clone());
		let secret = builder.secret().await?;
		Ok(match secret {
			Some(secret) => {
				let builtin_cores = Cores::default()
					.built_in_native_mapping()
					.into_iter()
					.map(|(cid, _)| cid)
					.collect();
				let encrypted_storage =
					EncryptedBlockStorage::new(storage.clone(), secret.into(), Default::default(), Default::default())
						.with_encryption_reference_mode(EncryptionReferenceMode::DisallowExcept(builtin_cores));
				for state in membership.state {
					if let Some(encryption_mapping) = &state.encryption_mapping {
						encrypted_storage.load_mapping(encryption_mapping).await?;
					}
				}
				ReducerStorage::Encrypted(CoStorage::new(encrypted_storage.clone()), encrypted_storage)
			},
			None => ReducerStorage::Default(storage.clone()),
		})
	}
}
impl AsRef<CoStorage> for ReducerStorage {
	fn as_ref(&self) -> &CoStorage {
		self.storage()
	}
}

#[derive(Clone)]
struct CoContextMembershipInstanceRegistry {
	reducers: ReducersControl,
}
#[async_trait]
impl MembershipInstanceRegistry for CoContextMembershipInstanceRegistry {
	async fn update(&self, co: CoId) -> Result<(), anyhow::Error> {
		if let Some(co_reducer) = self.reducers.reducer(co.clone()).await.ok() {
			if let Some(parent) = co_reducer.parent_id() {
				if let Some(parent_co_reducer) = self.reducers.reducer(parent.clone()).await.ok() {
					let context = co_reducer.context.clone();
					context.refresh(parent_co_reducer, co_reducer).await?;
				}
			}
		}
		Ok(())
	}

	async fn remove(&self, co: CoId) -> Result<(), anyhow::Error> {
		self.reducers.clone().clear_one(co).await?;
		Ok(())
	}
}
