use crate::cli::Cli;
use co_sdk::{Application, ApplicationBuilder};
use tokio_util::task::TaskTracker;

#[derive(Debug, Default, Clone)]
pub struct CliContext {
	pub(crate) tasks: TaskTracker,
}
impl CliContext {
	/// Create a new application instance.
	///
	/// Panics:
	/// - When the application could not be created.
	#[tracing::instrument(skip(self, cli))]
	pub async fn application(&self, cli: &Cli) -> Application {
		let mut application_builder = match &cli.base_path {
			None => ApplicationBuilder::new(cli.instance_id.to_owned()),
			Some(path) => ApplicationBuilder::new_with_path(cli.instance_id.to_owned(), path.clone()),
		};
		if cli.no_keychain {
			application_builder = application_builder.without_keychain();
		}
		let application = application_builder.build().await.expect("application");

		// add the application to cli task list
		let application_tasks = application.task_tracker();
		self.tasks.spawn(async move { application_tasks.wait().await });

		application
	}
}
