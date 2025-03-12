use super::{
	co_context::CoContext,
	identity::resolve_private_identity,
	shared::{CreateCo, SharedCoCreator},
	tracing::TracingBuilder,
};
use crate::{
	services::application::ApplicationMessage, Action, CoReducer, CoReducerFactory, CoStorage, Storage,
	CO_CORE_NAME_KEYSTORE, CO_CORE_NAME_MEMBERSHIP, CO_CORE_NAME_STORAGE,
};
use anyhow::anyhow;
use co_actor::{Actor, ActorHandle, ActorInstance};
use co_identity::{
	IdentityResolverBox, LocalIdentity, PrivateIdentity, PrivateIdentityBox, PrivateIdentityResolverBox,
};
use co_primitives::{tags, CoId, TagValue, Tags};
use directories::ProjectDirs;
use futures::{Stream, StreamExt};
use std::{fmt::Debug, future::ready, path::PathBuf, sync::Arc};
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

	// Tasks.
	tasks: TaskTracker,

	// CO Context.
	co_context: CoContext,

	/// The actor runtime.
	service: Arc<ActorInstance<crate::services::application::Application>>,
}
impl Application {
	pub fn settings(&self) -> &ApplicationSettings {
		&self.settings
	}

	pub fn storage(&self) -> CoStorage {
		self.context().inner.storage()
	}

	pub fn actions(&self) -> impl Stream<Item = Action> + Send + 'static {
		self.service
			.handle()
			.stream(ApplicationMessage::Subscribe)
			.filter_map(|item| ready(item.ok()))
	}

	pub fn shutdown(&self) -> CancellationToken {
		self.context().inner.shutdown().child_token()
	}

	pub fn handle(&self) -> ActorHandle<ApplicationMessage> {
		self.service.handle()
	}

	/// Tasks bound to this application.
	/// Internal use only. Use `tasks`.
	#[doc(hidden)]
	pub fn task_tracker(&self) -> TaskTracker {
		self.tasks.clone()
	}

	pub fn context(&self) -> &CoContext {
		&self.co_context
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
		self.context().inner.shutdown().cancel();

		// wait
		self.tasks.wait().await;
	}

	/// Create and startup network.
	pub async fn create_network(&mut self, force_new_peer_id: bool) -> Result<(), anyhow::Error> {
		// start
		self.service.handle().dispatch(Action::NetworkStart { force_new_peer_id })?;

		// wait
		let network = self.service.handle().request(ApplicationMessage::Network).await??;
		network.initialized().await?;

		// done
		Ok(())
	}

	/// Access Identity.
	///
	/// Todo: Identity Permissions?
	pub async fn private_identity(&self, did: &co_primitives::Did) -> Result<PrivateIdentityBox, anyhow::Error> {
		resolve_private_identity(&self.co_context, did).await
	}

	/// Identities.
	///
	/// Todo: Identity Permissions?
	pub async fn identity_resolver(&self) -> Result<IdentityResolverBox, anyhow::Error> {
		self.co_context.identity_resolver().await
	}

	/// Private Identities.
	///
	/// Todo: Identity Permissions?
	pub async fn private_identity_resolver(&self) -> Result<PrivateIdentityResolverBox, anyhow::Error> {
		self.co_context.private_identity_resolver().await
	}

	/// Get unsiged local device identity.
	pub fn local_identity(&self) -> LocalIdentity {
		self.co_context.local_identity()
	}

	/// Create a new CO.
	///
	/// TODO: Identity
	/// TODO: The crator of the co should be added as first participant.
	#[tracing::instrument(err, skip(self))]
	pub async fn create_co<I>(&self, creator: I, create: CreateCo) -> Result<CoReducer, anyhow::Error>
	where
		I: PrivateIdentity + Clone + Debug + Send + Sync + 'static,
	{
		// local
		let local = self.co_context.local_co_reducer().await?;

		// create
		let co = SharedCoCreator::new(local, create)
			.with_membership_core_name(CO_CORE_NAME_MEMBERSHIP.to_owned())
			.with_keystore_core_name(CO_CORE_NAME_KEYSTORE.to_owned())
			.with_storage_core_name(CO_CORE_NAME_STORAGE.to_owned())
			.create(self.storage(), self.context().inner.runtime(), creator)
			.await?;

		// load
		self.co().co_reducer(&co).await?.ok_or(anyhow!("Open CO failed: {}", co))
	}

	/// Initialize application.
	async fn init(&self) -> Result<(), anyhow::Error> {
		// shutdown
		let shutdown = self.context().inner.shutdown().clone();
		let tasks = self.tasks.clone();
		let reactive = self.service.handle();
		tokio::spawn(async move {
			// shutdown
			shutdown.cancelled().await;
			reactive.shutdown();
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
#[non_exhaustive]
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

	/// Extra settings.
	///
	/// Known Tags:
	/// - `co-local-watch` = `true` - [`TagValue::Bool`] Disable locals watcher.
	/// - `co-local-max-state` = `100` - [`TagValue::Integer`] Count of states to store for LocalCO. A value of zero
	///   means unlimited.
	/// - `co-local-max-log` = `100` - [`TagValue::Integer`] Count of transactions to store for LocalCO. A value of
	///   zero means unlimited.
	pub settings: Tags,
}

pub struct ApplicationBuilder {
	identifier: String,
	path: Option<PathBuf>,
	keychain: bool,
	tracing: TracingBuilder,
	settings: Tags,
}
impl ApplicationBuilder {
	pub fn default_path() -> PathBuf {
		let dirs = ProjectDirs::from("co.app", "1io", "co").expect("home directory");
		dirs.data_dir().into()
	}

	/// Create new instance with path.
	pub fn new_with_path(identifier: String, path: PathBuf) -> Self {
		let tracing = TracingBuilder::new(identifier.clone(), Some(path.clone()));
		Self { identifier, path: Some(path), keychain: true, tracing, settings: Default::default() }
	}

	pub fn new(identifier: String) -> Self {
		Self::new_with_path(identifier, Self::default_path())
	}

	/// Create new memory only instance.
	pub fn new_memory(identifier: String) -> Self {
		let tracing = TracingBuilder::new(identifier.clone(), None);
		Self { identifier, path: None, keychain: false, tracing, settings: Default::default() }
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

	/// See: [`ApplicationSettings::settings`]
	pub fn with_setting(self, name: &str, value: impl Into<TagValue>) -> Self {
		let mut settings = self.settings;
		settings.insert((name.to_owned(), value.into()));
		Self { settings, ..self }
	}

	pub async fn build(self) -> Result<Application, anyhow::Error> {
		let tasks = TaskTracker::new();

		// log
		self.tracing.init()?;

		// storage
		let storage = match &self.path {
			Some(path) => Storage::new(path.join("data"), path.join("tmp/data")),
			None => Storage::new_memory(),
		};

		// settings
		let settings = ApplicationSettings {
			application_path: self.path.map(|path| path.join("etc").join(&self.identifier)),
			identifier: self.identifier,
			keychain: self.keychain,
			settings: self.settings,
		};

		// create
		let service = Actor::spawn(
			tags!("type": "application", "application": settings.identifier.clone()),
			crate::services::application::Application::new(settings.clone()),
			(storage, tasks.clone()),
		)?;

		// wait for context
		let co_context = service.handle().request(ApplicationMessage::Context).await?;

		// instance
		let result = Application {
			_drop: Some(Arc::new(co_context.inner.shutdown().clone().drop_guard())),
			settings,
			co_context,
			service: Arc::new(service),
			tasks,
		};

		// init
		result.init().await?;

		Ok(result)
	}
}
