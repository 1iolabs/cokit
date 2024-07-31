use crate::{CoError, CoErrorSignal, CoSettings};
use anyhow::Result;
use co_sdk::{Application, ApplicationBuilder};
use dioxus::signals::Writable;
use futures::{future::BoxFuture, Future};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[derive(Debug, Clone)]
pub struct CoContext {
	tasks: UnboundedSender<Task>,
}
impl CoContext {
	pub(crate) fn new(settings: CoSettings) -> Self {
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
				application.tasks().spawn(async move {
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
}

type TaskFn = dyn (FnOnce(Application) -> BoxFuture<'static, ()>) + Send + 'static;
type Task = Box<TaskFn>;
// type Task = Box<dyn FnOnce(&Application) + Send + 'static>;

async fn co_app(settings: CoSettings, mut tasks: UnboundedReceiver<Task>) -> Result<(), anyhow::Error> {
	let identifier = settings.identifier;
	let builder = match settings.path {
		Some(path) => ApplicationBuilder::new_with_path(identifier, path),
		None => ApplicationBuilder::new(identifier),
	};
	let mut application = builder.without_keychain().with_bunyan_logging(None).build().await?;

	// network
	if settings.network {
		application.create_network(settings.network_force_new_peer_id).await?;
	}

	// execute
	tracing::info!("co-startup");
	while let Some(task) = tasks.recv().await {
		tracing::info!("co-task");
		task(application.clone()).await;
	}
	tracing::info!("co-shutdown");

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
