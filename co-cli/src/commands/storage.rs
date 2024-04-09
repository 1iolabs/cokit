mod cat;

use crate::cli::Cli;
use exitcode::ExitCode;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// CO Command
	#[command(subcommand)]
	pub command: Commands,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum Commands {
	/// Print a block.
	Cat(cat::Command),
}

pub async fn command(cli: &Cli, co_command: &Command) -> Result<ExitCode, anyhow::Error> {
	match &co_command.command {
		Commands::Cat(command) => cat::command(cli, command).await,
	}
}
