use crate::{cli::Cli, library::cli_context::CliContext};
use exitcode::ExitCode;

mod invite;
mod ls;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// DID Command
	#[command(subcommand)]
	pub command: Commands,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum Commands {
	/// List identities.
	Ls(ls::Command),

	/// Invite participant to an CO.
	Invite(invite::Command),
}

pub async fn command(context: &CliContext, cli: &Cli, did_command: &Command) -> Result<ExitCode, anyhow::Error> {
	match &did_command.command {
		Commands::Ls(command) => ls::command(context, cli, did_command, command).await,
		Commands::Invite(command) => invite::command(context, cli, did_command, command).await,
	}
}
