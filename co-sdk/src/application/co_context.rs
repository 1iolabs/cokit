use super::{
	application::ApplicationSettings,
	identity::{create_identity_resolver, create_private_identity_resolver},
	shared::SharedCoBuilder,
};
use crate::{
	drivers::network::{token::CoToken, CoNetworkTaskSpawner},
	library::{
		find_co_secret::find_co_secret_by_membership, find_membership::memberships, override_peer_provider::Overrides,
	},
	reactive::context::ReactiveContext,
	reducer::core_resolver::{
		dynamic::DynamicCoreResolver,
		epic::ReactiveCoreResolver,
		log::LogCoreResolver,
		membership::{MembershipCoreResolver, MembershipInstanceRegistry},
	},
	types::{co_reducer::CoReducerContext, co_reducer_factory::CoReducerFactoryError},
	CoCoreResolver, CoReducer, CoReducerFactory, CoStorage, LocalCoBuilder, Runtime, TaskSpawner,
	CO_CORE_NAME_KEYSTORE, CO_CORE_NAME_MEMBERSHIP, CO_ID_LOCAL,
};
use anyhow::anyhow;
use async_trait::async_trait;
use co_core_membership::Membership;
use co_identity::{
	IdentityResolverBox, LocalIdentity, PrivateIdentity, PrivateIdentityResolver, PrivateIdentityResolverBox,
};
use co_log::EntryBlock;
use co_network::bitswap;
use co_primitives::{CoId, Did};
use co_storage::BlockStorage;
use futures::{
	channel::{mpsc, oneshot},
	join, SinkExt, Stream, StreamExt, TryStreamExt,
};
use libp2p::PeerId;
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
			self.inner
				.create_co_instance(local, co, initialized, None)
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
		// currently on the local co is not shared
		// the call is async to be future proof when we may need to check some state
		co.as_str() != CO_ID_LOCAL
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
	pub async fn network(&self) -> Option<CoNetworkTaskSpawner> {
		self.inner.network.read().await.clone()
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
#[async_trait]
impl bitswap::StorageResolver<CoStorage> for CoContext {
	async fn resolve_storage(
		&self,
		remote_peer: Option<&PeerId>,
		tokens: &[bitswap::Token],
	) -> Result<CoStorage, anyhow::Error> {
		// use CO from first valid token
		for token in tokens {
			match CoToken::from_bitswap_token(token) {
				Ok(co_token) => {
					// get co storage
					//  we only accept networking for Shared CO's
					//  we get the storage using shared_co_storage and not via the reducer because:
					//  - performance/privacy: do not initialize our reducer if someone else is requesting blocks
					//  - to prevent a deadlock in the join process of encrypted COs (initial fetch of state/heads
					//    blocks)
					let local = self.local_co_reducer().await?;
					if let Some(co_storage) = self.inner.shared_co_storage(&local, &co_token.body.1).await? {
						let secret = find_co_secret_by_membership(&local, &co_token.body.1).await?;

						// verify remote peer if the CO is encrypted and this is an non local request
						match (remote_peer, &secret) {
							(Some(remote_peer), Some(secret)) => {
								if !co_token.verify(secret, remote_peer) {
									// check next token
									tracing::trace!(co = ?co_token.body.1, "bitswap-resolve-storage-invalid");
									continue;
								}
							},
							_ => {},
						};

						// get storage
						tracing::trace!(co = ?co_token.body.1, "bitswap-resolve-storage-co");
						return Ok(co_storage);
					} else {
						tracing::trace!(co = ?co_token.body.1, "bitswap-resolve-storage-unknown-co");
					}
				},
				Err(err) => {
					tracing::trace!(?err, "bitswap-resolve-storage-parse-failed");
				},
			}
		}

		// use the root storage (unencrypted)
		tracing::trace!("bitswap-resolve-storage-root");
		Ok(self.inner.storage())
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
	Request(CoId, oneshot::Sender<Result<CoReducer, CoReducerFactoryError>>),
	Create(CoId, Result<CoReducer, CoReducerFactoryError>),
	Clear(oneshot::Sender<Result<(), anyhow::Error>>),
	ClearOne(CoId, oneshot::Sender<Result<(), anyhow::Error>>),
}

#[derive(Clone)]
pub struct ReducersControl {
	sender: mpsc::Sender<ReducerRequest>,
}
impl ReducersControl {
	pub async fn reducer(&mut self, co: CoId) -> Result<CoReducer, CoReducerFactoryError> {
		let (tx, rx) = oneshot::channel();
		let (recv, send) = join!(rx, self.sender.send(ReducerRequest::Request(co, tx)));
		send.map_err(|err| CoReducerFactoryError::Other(err.into()))?;
		Ok(recv.map_err(|err| CoReducerFactoryError::Other(err.into()))??)
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
	pending_requests: VecDeque<ReducerRequest>,
	requests: (mpsc::Sender<ReducerRequest>, mpsc::Receiver<ReducerRequest>),
}
impl Reducers {
	pub fn new() -> (Self, ReducersControl) {
		let (tx, rx) = mpsc::channel(128);
		let reducers =
			Reducers { pending_requests: Default::default(), reducers: Default::default(), requests: (tx.clone(), rx) };
		(reducers, ReducersControl { sender: tx })
	}

	pub async fn worker(mut self, context: Arc<CoContextInner>) {
		let local_id = CoId::from(CO_ID_LOCAL);
		while let Some(request) = self.requests.1.next().await {
			match request {
				ReducerRequest::Request(id, response) => {
					// local
					let local = if let Some(local) = self.reducers.get(&local_id) {
						local
					} else {
						match context.create_local_co_instance(true).await {
							Ok(local) => {
								self.reducers.insert(local.id().clone(), local);
								self.reducers.get(&local_id).unwrap()
							},
							Err(err) => {
								response
									.send(Err(CoReducerFactoryError::Create(local_id.clone(), err.into())))
									.ok();
								continue;
							},
						}
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
								let mut sender = self.requests.0.clone();
								let parent = local.clone();
								async move {
									let create = match context.create_co_instance(parent, &id, true, None).await {
										Ok(Some(reducer)) => Ok(reducer),
										Ok(None) => Err(CoReducerFactoryError::CoNotFound(id.clone())),
										Err(err) => Err(CoReducerFactoryError::Create(id.clone(), err)),
									};
									sender.send(ReducerRequest::Create(id, create)).await.ok();
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
									Err(err) => Err(match err {
										CoReducerFactoryError::CoNotFound(id) => {
											CoReducerFactoryError::CoNotFound(id.clone())
										},
										CoReducerFactoryError::Create(id, err) => {
											CoReducerFactoryError::Create(id.to_owned(), anyhow!(err.to_string()))
										},
										CoReducerFactoryError::Other(err) => {
											CoReducerFactoryError::Other(anyhow!(err.to_string()))
										},
									}),
									Ok(reducer) => Ok(reducer.clone()),
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
}

#[derive(Clone)]
pub(crate) struct CoContextInner {
	settings: ApplicationSettings,

	shutdown: CancellationToken,
	tasks: TaskSpawner,

	local_identity: LocalIdentity,

	network: Arc<RwLock<Option<CoNetworkTaskSpawner>>>,
	network_overrides: Overrides,

	storage: CoStorage,
	runtime: Runtime,
	reactive_context: ReactiveContext,

	reducers: ReducersControl,
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
		reactive_context: ReactiveContext,
		reducers: ReducersControl,
	) -> Self {
		Self {
			settings,
			shutdown,
			tasks,
			local_identity,
			network: Arc::new(RwLock::new(network)),
			network_overrides: Default::default(),
			storage,
			runtime,
			reactive_context,
			reducers,
		}
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

	/// Clone with network.
	pub async fn set_network(&self, network: Option<CoNetworkTaskSpawner>) -> Result<(), anyhow::Error> {
		// assign
		*self.network.write().await = network;

		// clear reducers
		self.reducers.clone().clear().await?;

		// result
		Ok(())
	}

	/// Networking overrides.
	pub fn network_overrides(&self) -> Overrides {
		self.network_overrides.clone()
	}

	/// Creates a CoReducer instance of the Local CO.
	#[tracing::instrument(skip(self))]
	async fn create_local_co_instance(&self, initialize: bool) -> Result<CoReducer, anyhow::Error> {
		let core_resolver = CoCoreResolver::default();
		let core_resolver = ReactiveCoreResolver::<CoStorage, CoCoreResolver>::new(
			core_resolver,
			CO_ID_LOCAL.into(),
			&self.reactive_context,
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
			ReactiveCoreResolver::<CoStorage, CoCoreResolver>::new(core_resolver, id, &self.reactive_context);
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
		storage: Option<CoStorage>,
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
			.with_network_overrides(Some(self.network_overrides.clone()))
			.build(
				self.tasks.clone(),
				storage.unwrap_or_else(|| self.storage.clone()),
				self.runtime.clone(),
				identity,
				core_resolver,
			)
			.await?;

		// result
		Ok(reducer)
	}

	/// Creates a CoReducer instance a CO which we have a membership for.
	async fn create_co_instance(
		&self,
		parent: CoReducer,
		co: &CoId,
		initialize: bool,
		identity: Option<Did>,
	) -> Result<Option<CoReducer>, anyhow::Error> {
		// find first active membership
		let membership = self.shared_membership(&parent, co, identity.as_ref()).await?;
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
			self.create_co_instance_membership(parent, membership, identity, None, initialize, true)
				.await?,
		))
	}

	async fn shared_membership(
		&self,
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

	/// Get shared CO storage without initializing an reducer.
	async fn shared_co_storage(&self, parent: &CoReducer, co: &CoId) -> Result<Option<CoStorage>, anyhow::Error> {
		let membership = self.shared_membership(&parent, co, None).await?;
		if let Some(membership) = membership {
			let storage = SharedCoBuilder::new(parent.clone(), membership)
				.build_storage(self.storage())
				.await?;
			Ok(Some(storage))
		} else {
			Ok(None)
		}
	}
}
impl From<CoContextInner> for CoContext {
	fn from(val: CoContextInner) -> Self {
		CoContext { inner: Arc::new(val) }
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
