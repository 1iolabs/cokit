use crate::cli::{Cli, APP_IDENTIFIER};
use co_sdk::{Application, ApplicationBuilder};

#[tracing::instrument]
pub async fn application(cli: &Cli) -> Result<Application, anyhow::Error> {
	// application
	let mut application_builder = match &cli.base_path {
		None => ApplicationBuilder::new(APP_IDENTIFIER.to_owned()),
		Some(path) => ApplicationBuilder::new_with_path(APP_IDENTIFIER.to_owned(), path.clone()),
	};
	if cli.no_log == false {
		application_builder = application_builder.with_bunyan_logging(cli.log_path.clone());
	}
	if cli.no_keychain {
		application_builder = application_builder.without_keychain();
	}
	let application = application_builder.build().await.expect("application");

	// result
	Ok(application)
}
