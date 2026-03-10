// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::CoSettings;
use anyhow::Result;
use co_primitives::Network;
use co_sdk::{state, Application, ApplicationBuilder, CoId, Did, IdentityResolver};
use futures::{future::BoxFuture, Future};
use std::collections::{BTreeMap, BTreeSet};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
#[cfg(feature = "js")]
use tokio_with_wasm::alias as tokio;

#[derive(Debug, Clone)]
pub struct CoContext {
	tasks: UnboundedSender<Task>,
}
impl CoContext {
	pub fn new(settings: CoSettings) -> Self {
		match settings.log.clone().with_resolved_default() {
			#[cfg(feature = "web")]
			crate::CoLog::Console => {
				dioxus::logger::init(settings.log_level.into()).expect("logger");
			},
			#[cfg(feature = "tracing")]
			crate::CoLog::Print => {
				println!("tracing-print");
				co_sdk::TracingBuilder::new(settings.identifier.clone(), None)
					.with_stderr_logging()
					.with_max_level(settings.log_level.into())
					.init()
					.expect("tracing init");
			},
			#[cfg(all(feature = "fs", feature = "tracing"))]
			crate::CoLog::File(path) => {
				#[cfg(feature = "tracing")]
				{
					let base_path = match settings.storage.clone() {
						#[cfg(feature = "fs")]
						co_sdk::CoStorageSetting::Path(path) => Some(path),
						#[cfg(feature = "fs")]
						co_sdk::CoStorageSetting::PathDefault => Some(ApplicationBuilder::default_path()),
						_ => None,
					};
					println!("tracing-bunyan: {:?}", base_path);
					co_sdk::TracingBuilder::new(settings.identifier.clone(), base_path)
						.with_bunyan_logging(path)
						.with_max_level(settings.log_level.into())
						.init()
						.expect("tracing init");
				}
			},
			#[cfg(feature = "tracing-oslog")]
			crate::CoLog::Os => {
				println!("tracing-oslog");
				use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
				tracing_subscriber::registry()
					.with(
						tracing_subscriber::filter::EnvFilter::builder()
							.with_default_directive(
								tracing_subscriber::filter::LevelFilter::from_level(settings.log_level.into()).into(),
							)
							.from_env_lossy(),
					)
					.with(tracing_oslog::OsLogger::new(&settings.bundle_identifier, "default"))
					.init();
			},
			_ => {
				println!("tracing-none");
			},
		}

		// spawn
		Self::spawn(settings)
	}

	/// Wait until the context is ready.
	pub async fn ready(&self) -> Result<(), anyhow::Error> {
		let (tx, rx) = tokio::sync::oneshot::channel();
		self.execute(|_app| {
			tx.send(()).ok();
		});
		rx.await?;
		Ok(())
	}

	/// Wait until the context is ready by blocking.
	#[cfg(not(feature = "web"))]
	pub async fn ready_blocking(&self) -> Result<(), anyhow::Error> {
		let (tx, rx) = tokio::sync::oneshot::channel();
		self.execute(|_app| {
			tx.send(()).ok();
		});
		rx.blocking_recv()?;
		Ok(())
	}

	pub(crate) fn spawn(settings: CoSettings) -> Self {
		let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Task>();
		#[cfg(not(feature = "web"))]
		std::thread::Builder::new()
			.name("co".to_owned())
			.spawn(|| co_main(settings, rx))
			.expect("co thread to start");
		#[cfg(feature = "web")]
		co_main(settings, rx);
		Self { tasks: tx }
	}

	/// Execute future task using CO Application.
	/// Futures will be executed in sequence.
	pub(crate) fn execute_future_box<F>(&self, f: F)
	where
		F: (FnOnce(Application) -> BoxFuture<'static, ()>) + Send + 'static,
	{
		self.tasks
			.send(Box::new(f))
			.expect("co thread to run until all senders dropped");
	}

	/// Execute task using CO Application.
	pub(crate) fn execute<F>(&self, f: F)
	where
		F: FnOnce(Application) + Send + 'static,
	{
		self.execute_future_box(move |application| {
			Box::pin(async move {
				f(application);
			})
		});
	}

	/// Execute future task using CO Application.
	/// Futures will be started in sequence but executed in parallel.
	pub(crate) fn execute_future_parallel<F, Fut>(&self, f: F)
	where
		Fut: Future<Output = ()> + Send + 'static,
		F: FnOnce(Application) -> Fut + Send + 'static,
	{
		self.execute_future_box(move |application| {
			Box::pin(async move {
				application.context().tasks().spawn(async move {
					f(application).await;
				});
			})
		});
	}

	/// Execute future task using CO Application.
	/// Futures will be executed in sequence.
	pub(crate) fn execute_future<F, Fut>(&self, f: F)
	where
		Fut: Future<Output = ()> + Send + 'static,
		F: FnOnce(Application) -> Fut + Send + 'static,
	{
		self.execute_future_box(move |application| {
			Box::pin(async move {
				f(application).await;
			})
		});
	}

	/// Execute future task using CO Application and return its result when done.
	pub async fn try_with_application<F, Fut, T, E>(&self, f: F) -> Result<T, CoContextError<E>>
	where
		Fut: Future<Output = Result<T, E>> + Send + 'static,
		F: FnOnce(Application) -> Fut + Send + 'static,
		T: Send + 'static,
		E: Send + 'static,
	{
		let (tx, rx) = futures::channel::oneshot::channel();
		self.execute_future_box(move |application| {
			Box::pin(async move {
				let result = f(application).await;
				tx.send(result).ok();
			})
		});
		rx.await
			.map_err(|_err| CoContextError::Shutdown)?
			.map_err(|err| CoContextError::Execute(err))
	}

	/// Join a unrelated CO.
	///
	/// This only initiates a join.
	/// When completed the membership state of the CO will change to active.
	pub async fn join_unrelated_co(
		&self,
		from: state::Identity,
		to: Did,
		to_co: CoId,
		to_networks: BTreeSet<Network>,
	) -> Result<(), anyhow::Error> {
		Ok(self
			.try_with_application(move |application| async move {
				let to_identity = application.identity_resolver().await?.resolve(&to).await?;
				let from_identity = application.private_identity(&from.did).await?;
				co_sdk::join_unrelated_co(application.context(), &from_identity, &to_identity, to_co, to_networks)
					.await?;
				Result::<(), anyhow::Error>::Ok(())
			})
			.await?)
	}

	/// Send a contact request.
	pub async fn contact(
		&self,
		from: state::Identity,
		to: Did,
		to_subject: Option<String>,
		to_headers: BTreeMap<String, String>,
		to_networks: BTreeSet<Network>,
	) -> Result<(), anyhow::Error> {
		Ok(self
			.try_with_application(move |application| async move {
				application
					.context()
					.contact(from.did, to, to_subject, to_headers, to_networks)
					.await
			})
			.await?)
	}
}

#[derive(Debug, thiserror::Error)]
pub enum CoContextError<E> {
	#[error("Execute error")]
	Execute(E),

	#[error("Application has shutdown")]
	Shutdown,
}

type TaskFn = dyn (FnOnce(Application) -> BoxFuture<'static, ()>) + Send + 'static;
type Task = Box<TaskFn>;
// type Task = Box<dyn FnOnce(&Application) + Send + 'static>;

async fn co_app(settings: CoSettings, mut tasks: UnboundedReceiver<Task>) -> Result<(), anyhow::Error> {
	let mut application_builder = ApplicationBuilder::new_with_storage(settings.identifier, settings.storage)
		.with_cores(settings.cores)
		.with_guards(settings.guards);
	if settings.no_keychain {
		application_builder = application_builder.without_keychain();
	}
	if settings.no_default_features {
		application_builder = application_builder.with_setting("default-features", false);
	}
	for feature in &settings.feature {
		application_builder = application_builder.with_setting("feature", feature.to_owned());
	}
	if let Some(local_secret) = settings.local_secret {
		application_builder = application_builder.with_local_secret(local_secret);
	}
	if let Some(access_policy) = settings.access_policy {
		application_builder = application_builder.with_access_policy(access_policy);
	}
	if let Some(contact_handler) = settings.contact_handler {
		application_builder = application_builder.with_contact_handler(contact_handler);
	}
	#[allow(unused_mut)]
	let mut application = application_builder.build().await?;

	// network
	#[cfg(feature = "network")]
	if settings.network {
		application.create_network(settings.network_settings).await?;
	}

	// execute
	while let Some(task) = tasks.recv().await {
		task(application.clone()).await;
	}

	// result
	Ok(())
}

fn co_main(settings: CoSettings, tasks: UnboundedReceiver<Task>) {
	#[cfg(feature = "web")]
	wasm_bindgen_futures::spawn_local(async move {
		co_app(settings, tasks).await.expect("app to run");
	});

	#[cfg(not(feature = "web"))]
	tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build()
		.unwrap()
		.block_on(async move { co_app(settings, tasks).await.expect("app to run") });
}
