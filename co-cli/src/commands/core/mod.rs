// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{cli::Cli, library::cli_context::CliContext};
use exitcode::ExitCode;

mod build;
mod build_builtin;
mod inspect;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// COre Command.
	#[command(subcommand)]
	pub command: Commands,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum Commands {
	/// Build COre binary.
	Build(build::Command),

	/// Build built-on COre binaries.
	BuildBuiltin(build_builtin::Command),

	/// Inspect COre binary.
	Inspect(inspect::Command),
}

pub async fn command(context: &CliContext, cli: &Cli, core_command: &Command) -> Result<ExitCode, anyhow::Error> {
	match &core_command.command {
		Commands::Build(command) => build::command(context, cli, core_command, command).await,
		Commands::BuildBuiltin(command) => build_builtin::command(command).await,
		Commands::Inspect(command) => inspect::command(context, cli, core_command, command).await,
	}
}
