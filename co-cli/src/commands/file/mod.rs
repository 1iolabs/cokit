mod add;
mod cat;
mod ls;
mod mkdir;
mod rm;

use crate::cli::Cli;
use co_sdk::CoId;
use exitcode::ExitCode;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// The CO ID.
	pub co: CoId,

	/// The File Core Name.
	#[arg(long, default_value_t = String::from("file"))]
	pub core: String,

	/// File Command
	#[command(subcommand)]
	pub command: Commands,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum Commands {
	/// List directory contents.
	Ls(ls::Command),

	/// Create directory.
	Mkdir(mkdir::Command),

	/// Print file contents.
	Cat(cat::Command),

	/// Add new file.
	Add(add::Command),

	/// Remove file.
	Rm(rm::Command),
}

pub async fn command(cli: &Cli, file_command: &Command) -> Result<ExitCode, anyhow::Error> {
	match &file_command.command {
		Commands::Ls(command) => ls::command(cli, file_command, command).await,
		Commands::Mkdir(command) => mkdir::command(cli, file_command, command).await,
		Commands::Cat(command) => cat::command(cli, file_command, command).await,
		Commands::Add(command) => add::command(cli, file_command, command).await,
		Commands::Rm(command) => rm::command(cli, file_command, command).await,
	}
}
