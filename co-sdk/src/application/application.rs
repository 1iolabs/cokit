use super::shared::{CreateCo, SharedCoBuilder, SharedCoCreator};
use crate::{
	library::find_membership::find_membership, CoReducer, CoStorage, LocalCoBuilder, Runtime, Storage,
	CO_CORE_NAME_KEYSTORE, CO_CORE_NAME_MEMBERSHIP,
};
use anyhow::anyhow;
use co_log::{LocalIdentity, LocalIdentityResolver};
use co_runtime::RuntimePool;
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
		let local_co = LocalCoBuilder::new(
			self.identifier.clone(),
			self.application_path.clone(),
			self.keychain,
			self.local_identity(),
		);
		let local_co_reducer = local_co.build(self.storage(), self.runtime.clone()).await?;
		Ok(local_co_reducer)
	}

	/// Creates a CoReducer instance a CO which we have a membership for.
	///
	/// TODO: Identity
	async fn create_co_instance(&self, parent: CoReducer, co: &str) -> Result<Option<CoReducer>, anyhow::Error> {
		let membership = match find_membership(&parent, co).await? {
			Some(m) => m,
			None => return Ok(None),
		};
		let reducer = SharedCoBuilder::new(parent, membership)
			.with_membership_core_name(CO_CORE_NAME_MEMBERSHIP.to_owned())
			.with_keystore_core_name(CO_CORE_NAME_KEYSTORE.to_owned())
			.build(self.storage(), self.runtime.clone(), self.local_identity())
			.await?;
		Ok(Some(reducer))
	}

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

	/// Create a new CO.
	///
	/// TODO:
	/// - Identity
	/// - Cleanup when something fails?
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
				let log_file = std::fs::File::create(log_path)?;
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
