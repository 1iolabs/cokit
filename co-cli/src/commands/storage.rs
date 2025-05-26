mod cat;
mod gc;

use crate::{cli::Cli, library::cli_context::CliContext};
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

	/// Free unreferenced blocks.
	Gc(gc::Command),
}

pub async fn command(context: &CliContext, cli: &Cli, co_command: &Command) -> Result<ExitCode, anyhow::Error> {
	match &co_command.command {
		Commands::Cat(command) => cat::command(context, cli, command).await,
		Commands::Gc(command) => gc::command(context, cli, command).await,
	}
}
