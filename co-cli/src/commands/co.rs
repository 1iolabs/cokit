mod cat;
mod create;
mod ls;
mod remove;

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
	/// List all local COs.
	Ls,

	/// Print a block.
	Cat(cat::Command),

	/// Create a new CO.
	Create(create::Command),

	/// Remove/Leave a CO.
	Remove(remove::Command),
}

pub async fn command(cli: &Cli, co_command: &Command) -> Result<ExitCode, anyhow::Error> {
	match &co_command.command {
		Commands::Ls => ls::command(cli).await,
		Commands::Cat(command) => cat::command(cli, command).await,
		Commands::Create(command) => create::command(cli, command).await,
		Commands::Remove(command) => remove::command(cli, command).await,
	}
}
