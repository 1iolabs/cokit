// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{cli::Cli, library::cli_context::CliContext};
use exitcode::ExitCode;

mod invite;
mod ls;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// DID Command
	#[command(subcommand)]
	pub command: Commands,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum Commands {
	/// List identities.
	Ls(ls::Command),

	/// Invite participant to an CO.
	Invite(invite::Command),
}

pub async fn command(context: &CliContext, cli: &Cli, did_command: &Command) -> Result<ExitCode, anyhow::Error> {
	match &did_command.command {
		Commands::Ls(command) => ls::command(context, cli, did_command, command).await,
		Commands::Invite(command) => invite::command(context, cli, did_command, command).await,
	}
}
