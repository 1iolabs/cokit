use crate::{cli::Cli, library::cli_context::CliContext};
use exitcode::ExitCode;

mod check;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// COre Command.
	#[command(subcommand)]
	pub command: Commands,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum Commands {
	/// Check COre binary.
	Check(check::Command),
}

pub async fn command(context: &CliContext, cli: &Cli, core_command: &Command) -> Result<ExitCode, anyhow::Error> {
	match &core_command.command {
		Commands::Check(command) => check::command(context, cli, core_command, command).await,
	}
}
