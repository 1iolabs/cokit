// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

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
