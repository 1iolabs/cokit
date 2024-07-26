use crate::{cli::Cli, library::cli_context::CliContext};
use co_sdk::{CoId, CO_CORE_NAME_KEYSTORE, CO_ID_LOCAL};
use exitcode::ExitCode;

mod ls;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// The CO ID.
	#[arg(long, default_value_t = CoId::from(CO_ID_LOCAL))]
	pub co: CoId,

	/// The COre Name.
	#[arg(long, default_value_t = String::from(CO_CORE_NAME_KEYSTORE))]
	pub core: String,

	/// DID Command
	#[command(subcommand)]
	pub command: Commands,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum Commands {
	/// List identities.
	Ls(ls::Command),
}

pub async fn command(context: &CliContext, cli: &Cli, did_command: &Command) -> Result<ExitCode, anyhow::Error> {
	match &did_command.command {
		Commands::Ls(command) => ls::command(context, cli, did_command, command).await,
	}
}
