use crate::{cli::Cli, library::cli_context::CliContext};
use exitcode::ExitCode;

mod listen;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// CO Command
	#[command(subcommand)]
	pub command: Commands,

	/// Force to create a new PeerId.
	#[arg(long, default_value_t = false)]
	pub force_new_peer_id: bool,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum Commands {
	/// Listen for connections.
	Listen(listen::Command),
}

pub async fn command(context: &CliContext, cli: &Cli, network_command: &Command) -> Result<ExitCode, anyhow::Error> {
	match &network_command.command {
		Commands::Listen(command) => listen::command(context, cli, &network_command, command).await,
	}
}
