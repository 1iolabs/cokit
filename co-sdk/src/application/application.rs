use super::{
	identity::{create_identity_resolver, resolve_private_identity},
	shared::{CreateCo, SharedCoBuilder, SharedCoCreator},
};
use crate::{
	drivers::network::heads::ReceivedHeadsNetworkTask, library::find_membership::find_membership, local_keypair_fetch,
	types::co_storage::CoBlockStorageContentMapping, CoReducer, CoReducerFactory, CoStorage, LocalCoBuilder, Network,
	Runtime, Storage, CO_CORE_NAME_KEYSTORE, CO_CORE_NAME_MEMBERSHIP,
};
use anyhow::anyhow;
use async_trait::async_trait;
use co_identity::{LocalIdentity, LocalIdentityResolver, PrivateIdentityBox};
use co_log::EntryBlock;
use co_primitives::CoId;
use co_runtime::RuntimePool;
use co_storage::BlockStorage;
use directories::ProjectDirs;
use futures::{Stream, TryStreamExt};
use std::{collections::BTreeMap, mem::swap, ops::DerefMut, path::PathBuf, sync::Arc};
use tokio::sync::RwLock;
use tokio_util::{
	sync::{CancellationToken, DropGuard},
	task::TaskTracker,
};
use tracing::{level_filters::LevelFilter, subscriber::set_global_default};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{layer::SubscriberExt, Registry};

#[derive(Clone)]
pub struct Application {
	/// The Unique Application Instance Identifier.
	/// The Identifier should be hardcoded in the application.
	///
	/// Warning: When the application can have mulitple instances you need to pass a different identifier for every
	/// instance.
	identifier: String,

	/// Shared storage path.
	/// This path can be shared between different application which use the co-sdk.
	/// This enables using some shared resources like the storage and the same local CO.
	storage_path: PathBuf,

	/// Application preferences path.
	application_path: PathBuf,

	/// Path for application logs. If None no logs will be produced.
	log: Logging,

	/// CO Storage Driver.
	storage: Storage,

	/// CO Runtime Driver.
	runtime: Runtime,

	/// CO Network Driver.
	network: Option<Network>,

	/// Loaded reducers.
	reducers: Arc<RwLock<BTreeMap<CoId, CoReducer>>>,

	/// Use keychain or file for Local CO.
	keychain: bool,

	/// Application shutdown token.
	shutdown: CancellationToken,

	/// Shutdown the application when last reference is dropped.
	_drop: Arc<DropGuard>,

	// Tasks.
	tasks: TaskTracker,
}
impl Application {
	pub fn identifier(&self) -> &str {
		&self.identifier
	}

	pub fn application_path(&self) -> &PathBuf {
		&self.application_path
	}

	pub fn storage_path(&self) -> &PathBuf {
		&self.storage_path
	}

	pub fn storage(&self) -> CoStorage {
		self.storage.storage().clone()
	}

	pub fn network(&self) -> Option<Network> {
		self.network.clone()
	}

	pub fn shutdown(&self) -> CancellationToken {
		self.shutdown.clone()
	}

	/// Tasks bound to this application.
	pub fn tasks(&self) -> TaskTracker {
		self.tasks.clone()
	}

	pub fn runtime(&self) -> &RuntimePool {
		self.runtime.runtime()
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
		let local_co = self.local_co_reducer().await?;
		let network_key = local_keypair_fetch(&self.identifier, &local_co, &local_identity, force_new_peer_id)
			.await
			.expect("peer-id");
		self.network = Some(Network::new(network_key, self.storage(), create_identity_resolver()));

		// clear reducers to rebuild them with network support after this
		// we only keep local as this has no network
		{
			let mut reducers = self.reducers.write().await;
			let mut next_reducers = BTreeMap::new();
			if let Some(local) = reducers.remove("local") {
				next_reducers.insert("local".into(), local);
			}
			swap(&mut next_reducers, reducers.deref_mut());
		}

		// to be able to receivce updates anytime we add a static heads handler
		self.network
			.as_ref()
			.unwrap()
			.spawner()
			.spawn(ReceivedHeadsNetworkTask::new(self.clone()))?;

		// done
		Ok(())
	}

	/// Creates a CoReducer instance of the Local CO.
	async fn create_local_co_instance(&self, initialize: bool) -> Result<CoReducer, anyhow::Error> {
		let local_co = LocalCoBuilder::new(
			self.identifier.clone(),
			self.application_path.clone(),
			self.keychain,
			self.local_identity(),
			initialize,
		);
		let local_co_reducer = local_co
			.build(self.storage(), self.runtime.clone(), self.shutdown(), self.tasks())
			.await?;
		Ok(local_co_reducer)
	}

	/// Creates a CoReducer instance a CO which we have a membership for.
	///
	/// TODO: Identity
	///   - Which identity should write to the parent co? If its local we are fine.
	async fn create_co_instance(
		&self,
		parent: CoReducer,
		co: &CoId,
		initialize: bool,
	) -> Result<Option<CoReducer>, anyhow::Error> {
		let membership = match find_membership(&parent, co).await? {
			Some(m) => m,
			None => return Ok(None),
		};
		let reducer = SharedCoBuilder::new(parent, membership)
			.with_membership_core_name(CO_CORE_NAME_MEMBERSHIP.to_owned())
			.with_keystore_core_name(CO_CORE_NAME_KEYSTORE.to_owned())
			.with_network(self.network.as_ref().map(|n| n.spawner()))
			.with_initialize(initialize)
			.build(self.storage(), self.runtime.clone(), self.local_identity())
			.await?;
		Ok(Some(reducer))
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

	/// Get instance of CoReducer.
	/// Returns None if `co` membership could not be found.
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
			self.create_co_instance(local, co, true).await?
		};

		// store
		if let Some(reducer_cache) = &reducer {
			self.reducers.write().await.insert(co.to_owned(), reducer_cache.clone());
		}

		// result
		Ok(reducer)
	}

	/// Get a stream to the log entries.
	/// Starting at the latest.
	pub async fn co_log_entries(
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
			self.create_local_co_instance(initialized).await?
		} else {
			let local = self.local_co_reducer().await?;
			self.create_co_instance(local, co, initialized)
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

	/// Create a new CO.
	///
	/// TODO: Identity
	///  The crator of the co should be added as first participant.
	#[tracing::instrument(err, skip(self))]
	pub async fn create_co(&self, create: CreateCo) -> Result<CoReducer, anyhow::Error> {
		// local
		let local = self.local_co_reducer().await?;

		// identity
		let identity = self.local_identity();

		// create
		let co = SharedCoCreator::new(local, create)
			.with_membership_core_name(CO_CORE_NAME_MEMBERSHIP.to_owned())
			.with_keystore_core_name(CO_CORE_NAME_KEYSTORE.to_owned())
			.create(self.storage(), self.runtime.clone(), identity)
			.await?;

		// load
		Ok(self.co_reducer(&co).await?.ok_or(anyhow!("Open CO failed: {}", co))?)
	}

	/// Initialize application.
	async fn init(&self) -> Result<(), anyhow::Error> {
		// log
		match &self.log {
			Logging::Bunyan(log_path) => {
				std::fs::create_dir_all(log_path.parent().ok_or(anyhow!("no parent"))?)?;
				let log_file = std::fs::File::options().append(true).create(true).open(log_path)?;
				// let formatting_layer = BunyanFormattingLayer::new("co-daemon".into(), std::io::stdout);
				let formatting_layer = BunyanFormattingLayer::new(self.identifier.clone().into(), log_file);
				let subscriber = Registry::default()
					.with(LevelFilter::TRACE)
					.with(JsonStorageLayer)
					.with(formatting_layer);
				set_global_default(subscriber)?;
				LogTracer::init()?;
			},
			_ => {},
		}

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
#[async_trait]
impl CoReducerFactory for Application {
	async fn co_reducer(&self, co: &CoId) -> Result<Option<CoReducer>, anyhow::Error> {
		Application::co_reducer(&self, co).await
	}
}
impl std::fmt::Debug for Application {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Application")
			.field("identifier", &self.identifier)
			.field("storage_path", &self.storage_path)
			.field("application_path", &self.application_path)
			.field("log", &self.log)
			// .field("storage", &self.storage)
			// .field("runtime", &self.runtime)
			// .field("network", &self.network)
			// .field("reducers", &self.reducers)
			// .field("keychain", &self.keychain)
			.finish()
	}
}

pub struct ApplicationBuilder {
	identifier: String,
	path: PathBuf,
	log: Logging,
	keychain: bool,
}
impl ApplicationBuilder {
	pub fn default_path() -> PathBuf {
		let dirs = ProjectDirs::from("co.app", "1io", "co").expect("home directory");
		dirs.data_dir().into()
	}

	/// Create new instance with path.
	pub fn new_with_path(identifier: String, path: PathBuf) -> Self {
		Self { identifier, path, log: Logging::None, keychain: true }
	}

	pub fn new(identifier: String) -> Self {
		Self::new_with_path(identifier, Self::default_path())
	}

	/// Enable bunyan logging to log_path.
	/// If no path is specified {path}/log/application.log is used.
	/// Command read without network stuff:
	/// ```sh
	/// tail -0f ~/Application\ Support/co.app/log/application.log | bunyan -c '!/^(libp2p|hickory_proto)/.test(this.target)'
	/// ```
	pub fn with_bunyan_logging(self, log_path: Option<PathBuf>) -> Self {
		let log_path = match log_path {
			Some(p) => p,
			//None => self.path.join("log").join(format!("{}.log", &self.identifier)),
			None => self.path.join("log").join("co.log"),
		};
		Self { log: Logging::Bunyan(log_path), ..self }
	}

	pub fn without_keychain(self) -> Self {
		Self { keychain: false, ..self }
	}

	pub async fn build(self) -> Result<Application, anyhow::Error> {
		let storage_path = self.path.join("data");
		let shutdown = CancellationToken::new();
		let result = Application {
			network: None,
			storage: Storage::new(storage_path.clone()),
			runtime: Runtime::new(),
			storage_path,
			application_path: self.path.join("etc").join(&self.identifier),
			identifier: self.identifier,
			log: self.log,
			keychain: self.keychain,
			reducers: Default::default(),
			_drop: Arc::new(shutdown.clone().drop_guard()),
			shutdown: shutdown.clone(),
			tasks: TaskTracker::new(),
		};
		result.init().await?;
		Ok(result)
	}
}

#[derive(Debug, Clone)]
enum Logging {
	None,
	Bunyan(PathBuf),
}
