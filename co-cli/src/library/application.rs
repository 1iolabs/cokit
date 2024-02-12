use crate::cli::{Cli, APP_IDENTIFIER};
use co_sdk::{Application, ApplicationBuilder};
use std::path::PathBuf;

#[tracing::instrument]
pub async fn application(cli: &Cli) -> Result<Application, anyhow::Error> {
	// application
	let mut application_builder = match &cli.base_path {
		None => ApplicationBuilder::new(APP_IDENTIFIER.to_owned()),
		Some(path) => ApplicationBuilder::new_with_path(APP_IDENTIFIER.to_owned(), path.clone()),
	};
	if cli.no_keychain {
		application_builder = application_builder.without_keychain();
	}
	let application = application_builder.build().await.expect("application");

	// result
	Ok(application)
}

pub fn log_path(cli: &Cli) -> PathBuf {
	if let Some(path) = &cli.log_path {
		return path.clone();
	}
	let base_path = if let Some(path) = &cli.base_path { path.clone() } else { ApplicationBuilder::default_path() };
	base_path.join("log/co.log")
}
