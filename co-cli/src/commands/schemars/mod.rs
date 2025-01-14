use crate::{cli::Cli, library::cli_context::CliContext};
use exitcode::ExitCode;

mod generate;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	#[command(subcommand)]
	pub command: Commands,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum Commands {
	/// Used to generate Json schemas of specified modules
	Generate(generate::Command),
}

pub async fn command(_context: &CliContext, _cli: &Cli, command: &Command) -> Result<ExitCode, anyhow::Error> {
	match &command.command {
		Commands::Generate(command) => generate::command(command).await,
	}
}
