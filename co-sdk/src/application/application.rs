use super::{
	co_context::{CoContext, CoContextInner},
	identity::{create_identity_resolver, resolve_private_identity},
	shared::{CreateCo, SharedCoCreator},
	tracing::TracingBuilder,
};
use crate::{
	drivers::network::tasks::received_heads::ReceivedHeadsNetworkTask,
	identity::co_private_identity_resolver::CoPrivateIdentityResolver, local_keypair_fetch, CoReducer,
	CoReducerFactory, CoStorage, Network, Runtime, Storage, CO_CORE_NAME_KEYSTORE, CO_CORE_NAME_MEMBERSHIP,
};
use anyhow::anyhow;
use co_identity::{LocalIdentity, LocalIdentityResolver, PrivateIdentity, PrivateIdentityBox, PrivateIdentityResolver};
use co_primitives::CoId;
use co_runtime::RuntimePool;
use directories::ProjectDirs;
use std::{fmt::Debug, path::PathBuf, sync::Arc};
use tokio_util::{
	sync::{CancellationToken, DropGuard},
	task::TaskTracker,
};

#[derive(Clone)]
pub struct Application {
	/// Shutdown the application when last reference is dropped.
	_drop: Option<Arc<DropGuard>>,

	/// Settings.
	settings: ApplicationSettings,

	/// CO Storage Driver.
	storage: Storage,

	/// CO Runtime Driver.
	runtime: Runtime,

	/// CO Network Driver.
	network: Option<Network>,

	/// Application shutdown token.
	shutdown: CancellationToken,

	// Tasks.
	tasks: TaskTracker,

	// CO Context.
	co_context: CoContext,
}
impl Application {
	pub fn settings(&self) -> &ApplicationSettings {
		&self.settings
	}

	pub fn storage(&self) -> CoStorage {
		self.storage.storage().clone()
	}

	pub fn network(&self) -> Option<Network> {
		self.network.clone()
	}

	pub fn shutdown(&self) -> CancellationToken {
		self.shutdown.child_token()
	}

	/// Tasks bound to this application.
	pub fn tasks(&self) -> TaskTracker {
		self.tasks.clone()
	}

	pub fn runtime(&self) -> Runtime {
		self.runtime.clone()
	}

	pub fn runtime_pool(&self) -> &RuntimePool {
		self.runtime.runtime()
	}

	pub fn co(&self) -> &CoContext {
		&self.co_context
	}

	pub async fn local_co_reducer(&self) -> Result<CoReducer, anyhow::Error> {
		self.co().local_co_reducer().await
	}

	pub async fn co_reducer(&self, co: impl AsRef<CoId>) -> Result<Option<CoReducer>, anyhow::Error> {
		self.co().co_reducer(co.as_ref()).await
	}

	/// Shutdown the application gracefully.
	pub async fn shutdown_application(&self) {
		// signal
		self.shutdown.cancel();

		// wait
		self.tasks.wait().await;
	}

	/// Create and startup network.
	pub async fn create_network(&mut self, force_new_peer_id: bool) -> Result<(), anyhow::Error> {
		// create network
		let local_identity = self.local_identity();
		let local_co = self.co_context.local_co_reducer().await?;
		let network_key = local_keypair_fetch(&self.settings.identifier, &local_co, &local_identity, force_new_peer_id)
			.await
			.expect("peer-id");
		let network = Network::new(
			network_key,
			self.storage(),
			create_identity_resolver(),
			CoPrivateIdentityResolver::new(self.co().to_owned()).boxed(),
		);

		// shutdown
		//  when the token has been triggered explicitly shutdown the network
		if let Some(shutdown_network) = network.shutdown().await {
			let shutdown = self.shutdown.child_token().cancelled_owned();
			tokio::spawn(async move {
				shutdown.await;
				shutdown_network.shutdown();
			});
		}

		// replace reducer factory to rebuild them with network support after this
		self.co_context = self.co_context.inner.with_network(Some(network.spawner())).await.into();

		// assign
		self.network = Some(network);

		// to be able to receive updates anytime we add a static heads handler
		self.network
			.as_ref()
			.unwrap()
			.spawner()
			.spawn(ReceivedHeadsNetworkTask::new(self.co().clone()))?;

		// done
		Ok(())
	}

	/// Access Identity.
	///
	/// Todo: Identity Permissions?
	pub async fn private_identity(&self, did: &co_primitives::Did) -> Result<PrivateIdentityBox, anyhow::Error> {
		resolve_private_identity(self, &did).await
	}

	/// Get unsiged local device identity.
	pub fn local_identity(&self) -> LocalIdentity {
		LocalIdentityResolver::default().private_identity("did:local:device").unwrap()
	}

	/// Create a new CO.
	///
	/// TODO: Identity
	/// TODO: The crator of the co should be added as first participant.
	#[tracing::instrument(err, skip(self))]
	pub async fn create_co<I>(&self, creator: I, create: CreateCo) -> Result<CoReducer, anyhow::Error>
	where
		I: PrivateIdentity + Debug + Send + Sync + 'static,
	{
		// local
		let local = self.co_context.local_co_reducer().await?;

		// create
		let co = SharedCoCreator::new(local, create)
			.with_membership_core_name(CO_CORE_NAME_MEMBERSHIP.to_owned())
			.with_keystore_core_name(CO_CORE_NAME_KEYSTORE.to_owned())
			.create(self.storage(), self.runtime.clone(), creator)
			.await?;

		// load
		Ok(self.co().co_reducer(&co).await?.ok_or(anyhow!("Open CO failed: {}", co))?)
	}

	/// Initialize application.
	async fn init(&self) -> Result<(), anyhow::Error> {
		// shutdown
		let shutdown = self.shutdown.clone();
		let tasks = self.tasks.clone();
		tokio::spawn(async move {
			// shutdown
			shutdown.cancelled().await;
			tasks.close();

			// log
			tracing::trace!("application-shutdown");
		});

		// log
		tracing::trace!("application-startup");

		// result
		Ok(())
	}
}
impl std::fmt::Debug for Application {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Application")
			.field("identifier", &self.settings.identifier)
			// .field("application_path", &self.application_path)
			// .field("storage", &self.storage)
			// .field("runtime", &self.runtime)
			// .field("network", &self.network)
			// .field("reducers", &self.reducers)
			// .field("keychain", &self.keychain)
			.finish()
	}
}

#[derive(Debug, Clone)]
pub struct ApplicationSettings {
	/// The Unique Application Instance Identifier.
	/// The Identifier should be hardcoded in the application.
	///
	/// Warning: When the application can have mulitple instances you need to pass a different identifier for every
	/// instance.
	pub identifier: String,

	/// Application preferences path.
	///
	/// Normally composed of `{base_path}/etc/{identifier}`.
	/// The Local CO read method tries to read states of all applications by searching for
	/// `{application_path}/../*/local.cbor` files.
	pub application_path: Option<PathBuf>,

	/// Use keychain or file for Local CO.
	pub keychain: bool,
}

pub struct ApplicationBuilder {
	identifier: String,
	path: Option<PathBuf>,
	keychain: bool,
	tracing: TracingBuilder,
}
impl ApplicationBuilder {
	pub fn default_path() -> PathBuf {
		let dirs = ProjectDirs::from("co.app", "1io", "co").expect("home directory");
		dirs.data_dir().into()
	}

	/// Create new instance with path.
	pub fn new_with_path(identifier: String, path: PathBuf) -> Self {
		let tracing = TracingBuilder::new(identifier.clone(), Some(path.clone()));
		Self { identifier, path: Some(path), keychain: true, tracing }
	}

	pub fn new(identifier: String) -> Self {
		Self::new_with_path(identifier, Self::default_path())
	}

	/// Create new memory only instance.
	pub fn new_memory(identifier: String) -> Self {
		let tracing = TracingBuilder::new(identifier.clone(), None);
		Self { identifier, path: None, keychain: false, tracing }
	}

	/// Enable bunyan logging to log_path.
	/// If no path is specified {path}/log/application.log is used.
	/// Command read without network stuff:
	/// ```sh
	/// tail -0f ~/Application\ Support/co.app/log/application.log | bunyan -c '!/^(libp2p|hickory_proto)/.test(this.target)'
	/// ```
	pub fn with_bunyan_logging(self, log_path: Option<PathBuf>) -> Self {
		Self { tracing: self.tracing.with_bunyan_logging(log_path), ..self }
	}

	pub fn with_open_telemetry(self, endpoint: impl Into<String>) -> Self {
		Self { tracing: self.tracing.with_open_telemetry(endpoint), ..self }
	}

	pub fn without_keychain(self) -> Self {
		Self { keychain: false, ..self }
	}

	pub async fn build(self) -> Result<Application, anyhow::Error> {
		let shutdown = CancellationToken::new();
		let tasks = TaskTracker::new();
		let local_identity = LocalIdentityResolver::default().private_identity("did:local:device").unwrap();
		let runtime = Runtime::new();

		// log
		self.tracing.init()?;

		// storage
		let storage = match &self.path {
			Some(path) => Storage::new(path.join("data")),
			None => Storage::new_memory(),
		};

		// settings
		let settings = ApplicationSettings {
			application_path: self.path.map(|path| path.join("etc").join(&self.identifier)),
			identifier: self.identifier,
			keychain: self.keychain,
		};

		// co
		let co_context = CoContextInner::new(
			settings.clone(),
			shutdown.child_token(),
			tasks.clone(),
			local_identity.clone(),
			None,
			storage.storage(),
			runtime.clone(),
		)
		.into();

		// instance
		let result = Application {
			settings,
			network: None,
			storage,
			runtime: Runtime::new(),
			co_context,
			_drop: Some(Arc::new(shutdown.clone().drop_guard())),
			shutdown,
			tasks,
		};

		// init
		result.init().await?;

		Ok(result)
	}
}
