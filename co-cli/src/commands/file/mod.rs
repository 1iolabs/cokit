mod ls;
mod mkdir;

use crate::cli::Cli;
use exitcode::ExitCode;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// The CO ID.
	pub co: String,

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
	// /// Print file contents.
	// Cat(cat::Command),

	// /// Create a new CO.
	// Create(create::Command),

	// /// Remove/Leave a CO.
	// Remove(remove::Command),
}

pub async fn command(cli: &Cli, file_command: &Command) -> Result<ExitCode, anyhow::Error> {
	match &file_command.command {
		Commands::Ls(command) => ls::command(cli, file_command, command).await,
		Commands::Mkdir(command) => mkdir::command(cli, file_command, command).await,
		// Commands::Cat(command) => cat::command(cli, command).await,
		// Commands::Create(command) => create::command(cli, command).await,
		// Commands::Remove(command) => remove::command(cli, command).await,
	}
}
