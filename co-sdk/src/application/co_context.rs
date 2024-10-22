use super::{
	application::ApplicationSettings,
	identity::{create_identity_resolver, create_private_identity_resolver},
	shared::SharedCoBuilder,
};
use crate::{
	drivers::network::CoNetworkTaskSpawner,
	library::find_membership::memberships,
	reducer::core_resolver::{
		dynamic::DynamicCoreResolver,
		epic::ReactiveCoreResolver,
		log::LogCoreResolver,
		membership::{MembershipCoreResolver, MembershipInstanceRegistry},
	},
	services::{application::ApplicationMessage, connections::ConnectionMessage},
	types::{co_reducer::CoReducerContext, co_reducer_factory::CoReducerFactoryError},
	CoCoreResolver, CoReducer, CoReducerFactory, CoStorage, LocalCoBuilder, Runtime, TaskSpawner,
	CO_CORE_NAME_KEYSTORE, CO_CORE_NAME_MEMBERSHIP, CO_ID_LOCAL,
};
use anyhow::anyhow;
use async_trait::async_trait;
use co_actor::ActorHandle;
use co_core_membership::Membership;
use co_identity::{
	IdentityResolverBox, LocalIdentity, PrivateIdentity, PrivateIdentityResolver, PrivateIdentityResolverBox,
};
use co_log::EntryBlock;
use co_primitives::{CoId, Did};
use co_storage::{BlockStorage, EncryptedBlockStorage, StorageError};
use futures::{
	channel::{mpsc, oneshot},
	join, select, FutureExt, SinkExt, Stream, StreamExt, TryStreamExt,
};
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

	/// Applciation identifier.
	pub fn identifier(&self) -> &str {
		&self.inner.settings.identifier
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

enum ReducerRequest {
	/// Request CO reducer instance by creating it if not created yet.
	Request(CoId, oneshot::Sender<Result<CoReducer, CoReducerFactoryError>>),

	/// Create reducer instance.
	Create(CoId, Result<CoReducer, CoReducerFactoryError>),

	/// Clear all reducer instances.
	Clear(oneshot::Sender<Result<(), anyhow::Error>>),

	/// Clear a specific reducer instance.
	ClearOne(CoId, oneshot::Sender<Result<(), anyhow::Error>>),

	/// Request CO storage instance (without networking).
	Storage(CoId, oneshot::Sender<Result<ReducerStorage, CoReducerFactoryError>>),

	/// Create shared storage instance.
	CreateStorage(CoId, Result<ReducerStorage, CoReducerFactoryError>),
}

#[derive(Clone)]
pub struct ReducersControl {
	sender: mpsc::Sender<ReducerRequest>,
}
impl ReducersControl {
	pub async fn storage(&mut self, co: CoId) -> Result<ReducerStorage, CoReducerFactoryError> {
		// tracing::trace!(?co, err = ?anyhow::anyhow!("test"), "co-reducer-request");
		let (tx, rx) = oneshot::channel();
		let (recv, send) = join!(rx, self.sender.send(ReducerRequest::Storage(co, tx)));
		send.map_err(|err| CoReducerFactoryError::Other(err.into()))?;
		Ok(recv.map_err(|err| CoReducerFactoryError::Other(err.into()))??)
	}

	pub async fn reducer(&mut self, co: CoId) -> Result<CoReducer, CoReducerFactoryError> {
		// tracing::trace!(?co, err = ?anyhow::anyhow!("test"), "co-reducer-request");
		let (tx, rx) = oneshot::channel();
		let (recv, send) = join!(rx, self.sender.send(ReducerRequest::Request(co, tx)));
		send.map_err(|err| CoReducerFactoryError::Other(err.into()))?;
		Ok(recv.map_err(|err| CoReducerFactoryError::Other(err.into()))??)
	}

	pub async fn create(&mut self, co: CoId, reducer: Result<CoReducer, CoReducerFactoryError>) {
		self.sender.send(ReducerRequest::Create(co, reducer)).await.ok();
	}

	pub async fn create_storage(&mut self, co: CoId, storage: Result<ReducerStorage, CoReducerFactoryError>) {
		self.sender.send(ReducerRequest::CreateStorage(co, storage)).await.ok();
	}

	pub async fn clear(&mut self) -> Result<(), anyhow::Error> {
		let (tx, rx) = oneshot::channel();
		let (recv, send) = join!(rx, self.sender.send(ReducerRequest::Clear(tx)));
		send?;
		Ok(recv??)
	}

	pub async fn clear_one(&mut self, co: CoId) -> Result<(), anyhow::Error> {
		let (tx, rx) = oneshot::channel();
		let (recv, send) = join!(rx, self.sender.send(ReducerRequest::ClearOne(co, tx)));
		send?;
		Ok(recv??)
	}
}

pub struct Reducers {
	reducers: BTreeMap<CoId, CoReducer>,
	storages: BTreeMap<CoId, ReducerStorage>,
	pending_requests: VecDeque<ReducerRequest>,
	requests: mpsc::Receiver<ReducerRequest>,
}
impl Reducers {
	pub fn new() -> (Self, ReducersControl) {
		let (tx, rx) = mpsc::channel(128);
		let reducers = Reducers {
			pending_requests: Default::default(),
			reducers: Default::default(),
			storages: Default::default(),
			requests: rx,
		};
		(reducers, ReducersControl { sender: tx })
	}

	async fn local(&mut self, context: &CoContextInner) -> Result<CoReducer, CoReducerFactoryError> {
		let local_id = CoId::from(CO_ID_LOCAL);
		let local = if let Some(local) = self.reducers.get(&local_id) {
			local.clone()
		} else {
			let local = context.create_local_co_instance(true).await?;
			self.reducers.insert(local.id().clone(), local.clone());
			local
		};
		Ok(local)
	}

	pub async fn worker(mut self, context: Arc<CoContextInner>) {
		while let Some(request) = select! {
			item = self.requests.next() => item,
			_ = context.shutdown().cancelled_owned().fuse() => None,
		} {
			match request {
				ReducerRequest::Storage(id, response) => {
					// local
					let local = match self.local(&context).await {
						Ok(local) => local,
						Err(err) => {
							response
								.send(Err(CoReducerFactoryError::Create(CoId::from(CO_ID_LOCAL), err.into())))
								.ok();
							continue;
						},
					};

					// get/create
					if let Some(storage) = self.storages.get(&id) {
						response.send(Ok(storage.clone())).ok();
					} else {
						self.pending_requests.push_back(ReducerRequest::Storage(id.clone(), response));
						if self.pending_storage_count(&id) == 1 {
							// create storage
							context.tasks.spawn({
								let context = context.clone();
								let parent = local.clone();
								async move {
									let result = ReducerStorage::from_id(context.storage(), parent, id.clone()).await;
									context.reducers.clone().create_storage(id, result).await;
								}
							});
						}
					}
				},
				ReducerRequest::Request(id, response) => {
					// local
					let local = match self.local(&context).await {
						Ok(local) => local,
						Err(err) => {
							response
								.send(Err(CoReducerFactoryError::Create(CoId::from(CO_ID_LOCAL), err.into())))
								.ok();
							continue;
						},
					};

					// get/create
					if let Some(reducer) = self.reducers.get(&id) {
						response.send(Ok(reducer.clone())).ok();
					} else {
						self.pending_requests.push_back(ReducerRequest::Request(id.clone(), response));
						if self.pending_request_count(&id) == 1 {
							// create shared co
							context.tasks.spawn({
								let context = context.clone();
								let parent = local.clone();
								async move {
									// get storage
									let result = match context.reducers.clone().storage(id.clone()).await {
										Ok(storage) => {
											// create reducer
											match context.create_co_instance(parent, &id, storage, true, None).await {
												Ok(Some(reducer)) => Ok(reducer),
												Ok(None) => Err(CoReducerFactoryError::CoNotFound(id.clone())),
												Err(err) => Err(CoReducerFactoryError::Create(id.clone(), err)),
											}
										},
										Err(err) => Err(err),
									};

									// notify
									context.reducers.clone().create(id, result).await;
								}
							});
						}
					}
				},
				ReducerRequest::Clear(response) => {
					self.reducers.retain(|id, _| id.as_str() == CO_ID_LOCAL);
					response.send(Ok(())).ok();
				},
				ReducerRequest::ClearOne(id, response) => {
					self.reducers.retain(|retain_id, _| retain_id != &id);
					response.send(Ok(())).ok();
				},
				ReducerRequest::Create(id, result) => {
					// register
					match &result {
						Ok(reducer) => {
							self.reducers.insert(reducer.id().clone(), reducer.clone());
						},
						Err(err) => {
							tracing::error!(co = ?id, ?err, "co-reducer-failed");
						},
					}

					// respond pending
					let remove = self
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
						if let Some(ReducerRequest::Request(_, response)) = self.pending_requests.remove(index) {
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
							self.storages.insert(id.clone(), storage.clone());
						},
						Err(err) => {
							tracing::error!(co = ?id, ?err, "co-storage-failed");
						},
					}

					// respond pending
					let remove = self
						.pending_requests
						.iter()
						.enumerate()
						.filter_map(|(index, request)| match request {
							ReducerRequest::Storage(request_id, _) if request_id == &id => Some(index),
							_ => None,
						})
						.rev()
						.collect::<Vec<_>>();
					for index in remove {
						if let Some(ReducerRequest::Storage(_, response)) = self.pending_requests.remove(index) {
							response
								.send(match &result {
									Err(err) => Err(co_reducerfactory_error_clone(err)),
									Ok(storage) => Ok(storage.clone()),
								})
								.ok();
						}
					}
				},
			}
		}
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

fn co_reducerfactory_error_clone(err: &CoReducerFactoryError) -> CoReducerFactoryError {
	match err {
		CoReducerFactoryError::CoNotFound(id) => CoReducerFactoryError::CoNotFound(id.clone()),
		CoReducerFactoryError::Create(id, err) => {
			CoReducerFactoryError::Create(id.to_owned(), anyhow!(err.to_string()))
		},
		CoReducerFactoryError::Other(err) => CoReducerFactoryError::Other(anyhow!(err.to_string())),
	}
}

#[derive(Clone)]
pub(crate) struct CoContextInner {
	settings: ApplicationSettings,

	shutdown: CancellationToken,
	tasks: TaskSpawner,

	local_identity: LocalIdentity,

	network: Arc<RwLock<Option<(CoNetworkTaskSpawner, ActorHandle<ConnectionMessage>)>>>,

	storage: CoStorage,
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
		runtime: Runtime,
		reactive_context: ActorHandle<ApplicationMessage>,
		reducers: ReducersControl,
	) -> Self {
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
		}
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
	pub fn storage(&self) -> CoStorage {
		self.storage.clone()
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
		let core_resolver = CoCoreResolver::default();
		let core_resolver = ReactiveCoreResolver::<CoStorage, CoCoreResolver>::new(
			core_resolver,
			CO_ID_LOCAL.into(),
			self.reactive_context.clone(),
		);
		let core_resolver = MembershipCoreResolver::new(
			self.tasks.clone(),
			core_resolver,
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

	/// Creates the Core Resolver for a shared CO.
	pub(crate) fn create_co_core_resolver(&self, id: CoId) -> DynamicCoreResolver<CoStorage> {
		let core_resolver = CoCoreResolver::default();
		let core_resolver =
			ReactiveCoreResolver::<CoStorage, CoCoreResolver>::new(core_resolver, id, self.reactive_context.clone());
		let core_resolver = LogCoreResolver::new(core_resolver);
		let core_resolver = DynamicCoreResolver::new(core_resolver);
		core_resolver
	}

	/// Creates a CoReducer instance a CO.
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

#[derive(Debug, Clone)]
pub enum ReducerStorage {
	Default(CoStorage),
	Encrypted(CoStorage, EncryptedBlockStorage<CoStorage>),
}
impl ReducerStorage {
	pub fn storage(&self) -> CoStorage {
		match self {
			ReducerStorage::Default(storage) => storage.clone(),
			ReducerStorage::Encrypted(storage, _encrypted) => storage.clone(),
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
				let encrypted_storage =
					EncryptedBlockStorage::new(storage.clone(), secret.into(), Default::default(), Default::default());
				if let Some(encryption_mapping) = &membership.encryption_mapping {
					encrypted_storage.load_mapping(encryption_mapping).await?;
				}
				ReducerStorage::Encrypted(CoStorage::new(encrypted_storage.clone()), encrypted_storage)
			},
			None => ReducerStorage::Default(storage.clone()),
		})
	}
}

#[derive(Clone)]
struct CoContextMembershipInstanceRegistry {
	reducers: ReducersControl,
}
#[async_trait]
impl MembershipInstanceRegistry for CoContextMembershipInstanceRegistry {
	async fn update(&self, co: CoId) -> Result<(), anyhow::Error> {
		let mut reducers = self.reducers.clone();
		if let Some(co_reducer) = reducers.reducer(co.clone()).await.ok() {
			if let Some(parent) = co_reducer.parent_id() {
				if let Some(parent_co_reducer) = reducers.reducer(parent.clone()).await.ok() {
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
