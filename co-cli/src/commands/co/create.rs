// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{cli::Cli, library::cli_context::CliContext};
use co_sdk::{CoId, CreateCo};
use exitcode::ExitCode;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// CO ID
	pub co: CoId,

	/// CO Name
	pub name: Option<String>,

	/// Public (unencrypted)
	#[arg(short, default_value_t = false)]
	pub public: bool,
}

pub async fn command(context: &CliContext, cli: &Cli, command: &Command) -> Result<ExitCode, anyhow::Error> {
	let application = context.application(cli).await;

	// create
	let create = CreateCo::new(&command.co, command.name.clone()).with_public(command.public);
	let reducer = application.create_co(application.local_identity(), create).await?;

	// result
	println!("{} | {}", &command.co, reducer.reducer_state().await.0.expect("state"));

	// result
	Ok(exitcode::OK)
}
