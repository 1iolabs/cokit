// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

mod create;
mod edit;
mod get;
mod send;

use crate::{cli::Cli, library::cli_context::CliContext};
use co_sdk::CoId;
use exitcode::ExitCode;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// ID of the co
	pub co: CoId,

	/// The room core name
	#[arg(long, default_value_t = String::from("room"))]
	core: String,

	#[command(subcommand)]
	pub command: Commands,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum Commands {
	Create(create::Command),
	Send(send::Command),
	Get(get::Command),
	Edit(create::Command),
}

pub async fn command(context: &CliContext, cli: &Cli, room_command: &Command) -> Result<ExitCode, anyhow::Error> {
	match &room_command.command {
		Commands::Create(command) => create::command(context, cli, room_command, command).await,
		Commands::Send(command) => send::command(context, cli, room_command, command).await,
		Commands::Get(command) => get::command(context, cli, room_command, command).await,
		Commands::Edit(command) => edit::command(context, cli, room_command, command).await,
	}
}
