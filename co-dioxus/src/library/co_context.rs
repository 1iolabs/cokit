// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::CoSettings;
use anyhow::Result;
#[cfg(feature = "fs")]
use co_sdk::CoStorageSetting;
use co_sdk::{Application, ApplicationBuilder};
use futures::{future::BoxFuture, Future};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
#[cfg(feature = "js")]
use tokio_with_wasm::alias as tokio;

#[derive(Debug, Clone)]
pub struct CoContext {
	tasks: UnboundedSender<Task>,
}
impl CoContext {
	pub fn new(settings: CoSettings) -> Self {
		// log
		//  note: we initialize the logging synchonously before dioxus registers the default
		#[cfg(feature = "tracing")]
		if !settings.no_log {
			let base_path = match settings.storage.clone() {
				#[cfg(feature = "fs")]
				CoStorageSetting::Path(path) => Some(path),
				#[cfg(feature = "fs")]
				CoStorageSetting::PathDefault => Some(ApplicationBuilder::default_path()),
				_ => None,
			};
			co_sdk::TracingBuilder::new(settings.identifier.clone(), base_path)
				.with_bunyan_logging(None)
				.with_max_level(settings.log_level.into())
				.init()
				.expect("tracing init");
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
