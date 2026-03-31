// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

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
	#[tracing::instrument(level = tracing::Level::TRACE, skip(self, cli))]
	pub async fn application(&self, cli: &Cli) -> Application {
		let mut application_builder = match &cli.base_path {
			None => ApplicationBuilder::new(cli.instance_id.to_owned()),
			Some(path) => ApplicationBuilder::new_with_path(cli.instance_id.to_owned(), path.clone()),
		};
		if cli.no_keychain {
			application_builder = application_builder.without_keychain();
		}
		if cli.no_default_features {
			application_builder = application_builder.with_setting("default-features", false);
		}
		for feature in &cli.feature {
			application_builder = application_builder.with_setting("feature", feature.to_owned());
		}
		let application = application_builder.build().await.expect("application");

		// add the application to cli task list
		//  note: we only clone the tasks so the application gets closed when goes out of scope
		self.tasks.spawn(application.joiner());

		application
	}
}
