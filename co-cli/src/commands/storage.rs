// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

mod cat;
#[cfg(feature = "pinning")]
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
	#[cfg(feature = "pinning")]
	Gc(gc::Command),
}

pub async fn command(context: &CliContext, cli: &Cli, co_command: &Command) -> Result<ExitCode, anyhow::Error> {
	match &co_command.command {
		Commands::Cat(command) => cat::command(context, cli, command).await,
		#[cfg(feature = "pinning")]
		Commands::Gc(command) => gc::command(context, cli, command).await,
	}
}
