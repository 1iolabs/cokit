// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

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
