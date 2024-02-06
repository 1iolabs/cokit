use crate::{CoReducer, CoStorage, LocalCo, Runtime, Storage};
use co_runtime::RuntimePool;
use directories::ProjectDirs;
use std::{path::PathBuf, sync::Arc};
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

	/// Application state path.
	application_path: PathBuf,

	/// Path for application logs. If None no logs will be produced.
	log: Log,

	/// CO Storage Driver.
	storage: Arc<Storage>,

	/// CO Runtime Driver.
	runtime: Arc<Runtime>,
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

	pub async fn create_local_co(&self) -> Result<CoReducer, anyhow::Error> {
		let local_co = LocalCo::new(self.identifier.clone(), self.application_path.clone());
		let local_co_reducer = local_co.read(self.storage(), self.runtime()).await?;
		Ok(CoReducer { reducer: Arc::new(RwLock::new(local_co_reducer)), runtime: self.runtime.clone() })
	}

	/// Initialize application.
	///
	/// Panics:
	/// - Can not create/open log file.
	async fn init(&self) -> Result<(), anyhow::Error> {
		// log
		match &self.log {
			Log::Bunyan(log_path) => {
				std::fs::create_dir_all(log_path.parent().expect("not root")).expect("create folders");
				let log_file = std::fs::File::create(log_path).unwrap();
				// let formatting_layer = BunyanFormattingLayer::new("co-daemon".into(), std::io::stdout);
				let formatting_layer = BunyanFormattingLayer::new(self.identifier.clone().into(), log_file);
				let subscriber = Registry::default()
					.with(LevelFilter::INFO)
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
	log: Log,
}
impl ApplicationBuilder {
	/// Create new instance with path.
	pub fn new_with_path(identifier: String, path: PathBuf) -> Self {
		Self { identifier, path, log: Log::None }
	}

	pub fn new(identifier: String) -> Self {
		let dirs = ProjectDirs::from("co.app", "1io", "co").expect("home directory");
		Self { identifier, path: dirs.data_dir().into(), log: Log::None }
	}

	/// Enable bunyan logging to log_path.
	/// If no path is specified {path}/log/application.log is used.
	pub fn with_bunyan_logging(self, log_path: Option<PathBuf>) -> Self {
		let log_path = match log_path {
			Some(p) => p,
			//None => self.path.join("log").join(format!("{}.log", &self.identifier)),
			None => self.path.join("log").join("application.log"),
		};
		Self { log: Log::Bunyan(log_path), ..self }
	}

	pub async fn build(self) -> Result<Application, anyhow::Error> {
		let storage_path = self.path.join("data");
		let result = Application {
			storage: Arc::new(Storage::new(storage_path.clone())),
			runtime: Arc::new(Runtime::new()),
			storage_path,
			application_path: self.path.join("etc").join(&self.identifier),
			identifier: self.identifier,
			log: self.log,
		};
		result.init().await?;
		Ok(result)
	}
}

#[derive(Debug, Clone)]
enum Log {
	None,
	Bunyan(PathBuf),
}
