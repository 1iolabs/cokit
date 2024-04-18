use crate::{CoError, CoErrorSignal, CoSettings};
use co_sdk::{Application, ApplicationBuilder};
use dioxus::signals::Writable;
use futures::Future;
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

	/// Execute task using CO Application.
	pub fn execute<F>(&self, f: F)
	where
		F: FnOnce(&Application) + Send + 'static,
	{
		self.tasks
			.send(Box::new(f))
			.expect("co thread to run until all senders dropped");
	}

	pub fn execute_future_with_error<F, Fut>(&self, mut error: CoErrorSignal, f: F)
	where
		Fut: Future<Output = Result<(), anyhow::Error>> + Send + 'static,
		F: FnOnce(Application) -> Fut + Send + 'static,
	{
		self.execute(move |application| {
			let application = application.clone();
			application.tasks().spawn(async move {
				match f(application).await {
					Ok(_) => {},
					Err(e) => error.write().push(CoError::from_error(e)),
				}
			});
		});
	}
}

// type Task = Box<dyn FnOnce(&Application) -> futures::BoxFuture<()> + Send + 'static>;
type Task = Box<dyn FnOnce(&Application) + Send + 'static>;

async fn co_app(settings: CoSettings, mut tasks: UnboundedReceiver<Task>) {
	let identifier = settings.identifier;
	let builder = match settings.path {
		Some(path) => ApplicationBuilder::new_with_path(identifier, path),
		None => ApplicationBuilder::new(identifier),
	};
	let mut application = builder
		.without_keychain()
		.with_bunyan_logging(None)
		.build()
		.await
		.expect("application");

	// network
	if settings.network {
		application
			.create_network(settings.network_force_new_peer_id)
			.await
			.expect("network");
	}

	// execute
	tracing::info!("co-startup");
	while let Some(task) = tasks.recv().await {
		tracing::info!("co-task");
		task(&mut application);
	}
	tracing::info!("co-shutdown");
}

fn co_main(settings: CoSettings, tasks: UnboundedReceiver<Task>) {
	tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build()
		.unwrap()
		.block_on(async move { co_app(settings, tasks).await })
}
