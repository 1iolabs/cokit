use co_sdk::{Application, ApplicationBuilder};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[derive(Debug, Clone)]
pub struct CoContext {
	tasks: UnboundedSender<Task>,
}
impl CoContext {
	pub fn new() -> Self {
		let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Task>();
		std::thread::Builder::new()
			.name("co".to_owned())
			.spawn(|| co_main(rx))
			.expect("co thread to start");
		Self { tasks: tx }
	}

	pub fn execute<F>(&self, f: F)
	where
		F: FnOnce(&Application) + Send + 'static,
	{
		self.tasks
			.send(Box::new(f))
			.expect("co thread to run until all senders dropped");
	}
}

// type Task = Box<dyn FnOnce(&Application) -> futures::BoxFuture<()> + Send + 'static>;
type Task = Box<dyn FnOnce(&Application) + Send + 'static>;

async fn co_app(mut tasks: UnboundedReceiver<Task>) {
	let path = "/Users/dominik/Workspaces/test/co/data";
	let builder = ApplicationBuilder::new_with_path("dioxus".to_owned(), path.into());
	let mut application = builder
		.without_keychain()
		.with_bunyan_logging(None)
		.build()
		.await
		.expect("application");
	tracing::info!("co-startup");
	while let Some(task) = tasks.recv().await {
		tracing::info!("co-task");
		task(&mut application);
	}
	tracing::info!("co-shutdown");
}

fn co_main(tasks: UnboundedReceiver<Task>) {
	tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build()
		.unwrap()
		.block_on(async move { co_app(tasks).await })
}
