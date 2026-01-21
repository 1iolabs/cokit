mod cat;
mod create;
mod log;
mod ls;
mod remove;
mod show;

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
	/// List all local COs.
	Ls,

	/// Show CO details.
	Show(show::Command),

	/// Print a block.
	Cat(cat::Command),

	/// Create a new CO.
	Create(create::Command),

	/// Remove/Leave a CO.
	Remove(remove::Command),

	/// Print CO Log.
	Log(log::Command),
}

pub async fn command(context: &CliContext, cli: &Cli, co_command: &Command) -> Result<ExitCode, anyhow::Error> {
	match &co_command.command {
		Commands::Ls => ls::command(context, cli).await,
		Commands::Show(command) => show::command(context, cli, command).await,
		Commands::Cat(command) => cat::command(context, cli, command).await,
		Commands::Create(command) => create::command(context, cli, command).await,
		Commands::Remove(command) => remove::command(context, cli, command).await,
		Commands::Log(command) => log::command(context, cli, command).await,
	}
}
