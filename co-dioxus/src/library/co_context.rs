// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{CoError, CoErrorSignal, CoSettings};
use anyhow::Result;
use co_sdk::{Application, ApplicationBuilder};
use dioxus::signals::WritableExt;
use futures::{future::BoxFuture, Future};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[derive(Debug, Clone)]
pub struct CoContext {
	tasks: UnboundedSender<Task>,
}
impl CoContext {
	pub fn new(settings: CoSettings) -> Self {
		let context = Self::spawn(settings);

		// block until startup is complete
		let (tx, rx) = tokio::sync::oneshot::channel();
		context.execute(|_app| {
			tx.send(()).unwrap();
		});
		rx.blocking_recv().unwrap();

		// result
		context
	}

	pub(crate) fn spawn(settings: CoSettings) -> Self {
		let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Task>();
		std::thread::Builder::new()
			.name("co".to_owned())
			.spawn(|| co_main(settings, rx))
			.expect("co thread to start");
		Self { tasks: tx }
	}

	/// Execute future task using CO Application.
	/// Futures will be executed in sequence.
	pub fn execute_future_box<F>(&self, f: F)
	where
		F: (FnOnce(Application) -> BoxFuture<'static, ()>) + Send + 'static,
	{
		self.tasks
			.send(Box::new(f))
			.expect("co thread to run until all senders dropped");
	}

	/// Execute task using CO Application.
	pub fn execute<F>(&self, f: F)
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
	pub fn execute_future_parallel<F, Fut>(&self, f: F)
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
	pub fn execute_future<F, Fut>(&self, f: F)
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

	/// Execute future task using CO Application.
	/// Note: Tasks will be started in sequence but executed in parallel.
	pub fn execute_future_with_error<F, Fut>(&self, mut error: CoErrorSignal, f: F)
	where
		Fut: Future<Output = Result<(), anyhow::Error>> + Send + 'static,
		F: FnOnce(Application) -> Fut + Send + 'static,
	{
		self.execute_future_box(move |application| {
			Box::pin(async move {
				match f(application).await {
					Ok(_) => {},
					Err(e) => error.write().push(CoError::from_error(e)),
				}
			})
		});
	}

	/// Execute future task using CO Application and return its result when done.
	/// Note: Tasks will be started in sequence but executed in parallel.
	pub async fn result<F, Fut, T>(&self, mut error: CoErrorSignal, f: F) -> Option<T>
	where
		Fut: Future<Output = Result<T, anyhow::Error>> + Send + 'static,
		F: FnOnce(Application) -> Fut + Send + 'static,
		T: Send + 'static,
	{
		let (tx, rx) = futures::channel::oneshot::channel();
		self.execute_future_box(move |application| {
			Box::pin(async move {
				match f(application).await {
					Ok(result) => {
						tx.send(result).ok();
					},
					Err(e) => error.write().push(CoError::from_error(e)),
				}
			})
		});
		rx.await.ok()
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
	let mut application_builder = match settings.path {
		Some(path) => ApplicationBuilder::new_with_path(settings.identifier, path),
		None => ApplicationBuilder::new(settings.identifier),
	};
	if !settings.no_log {
		application_builder = application_builder.with_bunyan_logging(None);
	}
	if settings.no_keychain {
		application_builder = application_builder.without_keychain();
	}
	if settings.no_default_features {
		application_builder = application_builder.with_setting("default-features", false);
	}
	application_builder = application_builder.with_log_max_level(settings.log_level.into());
	for feature in &settings.feature {
		application_builder = application_builder.with_setting("feature", feature.to_owned());
	}
	application_builder = application_builder.with_setting("feature", "co-open-keep");
	let mut application = application_builder.build().await?;

	// network
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
	tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build()
		.unwrap()
		.block_on(async move { co_app(settings, tasks).await.expect("app to run") })
}
