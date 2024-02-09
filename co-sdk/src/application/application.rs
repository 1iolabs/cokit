use crate::{
	library::find_membership::find_membership, CoCoreResolver, CoReducer, CoStorage, LocalCo, ReducerBuilder, Runtime,
	Storage, CO_CORE_KEYSTORE,
};
use anyhow::anyhow;
use co_log::{LocalIdentityResolver, Log};
use co_runtime::RuntimePool;
use co_storage::{EncryptedBlockStorage, Secret};
use directories::ProjectDirs;
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
use tokio::sync::RwLock;
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

	/// Loaded reducers.
	reducers: Arc<RwLock<BTreeMap<String, CoReducer>>>,

	/// Use keychain or file for Local CO.
	keychain: bool,
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

	pub fn runtime(&self) -> &RuntimePool {
		self.runtime.runtime()
	}

	/// Creates a CoReducer instance of the Local CO.
	async fn create_local_co_instance(&self) -> Result<CoReducer, anyhow::Error> {
		let local_co = LocalCo::new(self.identifier.clone(), self.application_path.clone(), self.keychain);
		let local_co_reducer = local_co.into_reducer(self.storage(), self.runtime.clone()).await?;
		Ok(local_co_reducer)
	}

	/// Creates a CoReducer instance a CO which we have a membership for.
	async fn create_co_instance(&self, local: CoReducer, co: &str) -> Result<Option<CoReducer>, anyhow::Error> {
		let membership = match find_membership(local.clone(), co).await? {
			Some(m) => m,
			None => return Ok(None),
		};

		// storage
		let storage: CoStorage = match &membership.key {
			Some(key_reference) => {
				let key_store: co_core_keystore::KeyStore = local.state(CO_CORE_KEYSTORE).await?;
				let key = key_store
					.shared_key(key_reference)
					.ok_or(anyhow!("Shared key not found: {}", key_reference))?;
				CoStorage::new(EncryptedBlockStorage::new(self.storage(), Secret::new(key.clone()), Default::default()))
			},
			None => self.storage(),
		};

		// log
		let log = Log::new(
			co.as_bytes().to_vec(),
			LocalIdentityResolver::default().private_identity("did:local:device")?,
			Box::new(LocalIdentityResolver::default()),
			storage,
			membership.heads.clone(),
		);

		// reducer
		let reducer = ReducerBuilder::new(CoCoreResolver::default(), log)
			.with_latest_state(membership.state, membership.heads.clone())
			.build(self.runtime())
			.await?;

		// result
		Ok(Some(CoReducer::new(self.runtime.clone(), reducer)))
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
		let reducer = self.create_local_co_instance().await?;

		// store
		self.reducers.write().await.insert(co.to_owned(), reducer.clone());

		// result
		Ok(reducer)
	}

	/// Get instance of CoReducer.
	/// Returns None if `co` membership could not be found.
	pub async fn co_reducer(&self, co: &str) -> Result<Option<CoReducer>, anyhow::Error> {
		// has one?
		{
			let reducers = self.reducers.read().await;
			let reducer = reducers.get(co);
			if let Some(reducer) = reducer {
				return Ok(Some(reducer.clone()));
			}
		}

		// create
		let reducer = if co == "local" {
			Some(self.create_local_co_instance().await?)
		} else {
			let local = self.local_co_reducer().await?;
			self.create_co_instance(local, co).await?
		};

		// store
		if let Some(reducer_cache) = &reducer {
			self.reducers.write().await.insert(co.to_owned(), reducer_cache.clone());
		}

		// result
		Ok(reducer)
	}

	/// Initialize application.
	///
	/// Panics:
	/// - Can not create/open log file.
	async fn init(&self) -> Result<(), anyhow::Error> {
		// log
		match &self.log {
			Logging::Bunyan(log_path) => {
				std::fs::create_dir_all(log_path.parent().expect("not root")).expect("create folders");
				let log_file = std::fs::File::create(log_path).unwrap();
				// let formatting_layer = BunyanFormattingLayer::new("co-daemon".into(), std::io::stdout);
				let formatting_layer = BunyanFormattingLayer::new(self.identifier.clone().into(), log_file);
				let subscriber = Registry::default()
					.with(LevelFilter::TRACE)
					.with(JsonStorageLayer)
					.with(formatting_layer);
				set_global_default(subscriber).unwrap();
				LogTracer::init().unwrap();
			},
			_ => {},
		}

		// result
		Ok(())
	}
}

pub struct ApplicationBuilder {
	identifier: String,
	path: PathBuf,
	log: Logging,
	keychain: bool,
}
impl ApplicationBuilder {
	/// Create new instance with path.
	pub fn new_with_path(identifier: String, path: PathBuf) -> Self {
		Self { identifier, path, log: Logging::None, keychain: true }
	}

	pub fn new(identifier: String) -> Self {
		let dirs = ProjectDirs::from("co.app", "1io", "co").expect("home directory");
		Self::new_with_path(identifier, dirs.data_dir().into())
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
			None => self.path.join("log").join("application.log"),
		};
		Self { log: Logging::Bunyan(log_path), ..self }
	}

	pub fn without_keychain(self) -> Self {
		Self { keychain: false, ..self }
	}

	pub async fn build(self) -> Result<Application, anyhow::Error> {
		let storage_path = self.path.join("data");
		let result = Application {
			storage: Storage::new(storage_path.clone()),
			runtime: Runtime::new(),
			storage_path,
			application_path: self.path.join("etc").join(&self.identifier),
			identifier: self.identifier,
			log: self.log,
			keychain: self.keychain,
			reducers: Default::default(),
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
