// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{cli::Cli, library::cli_context::CliContext};
use exitcode::ExitCode;

mod listen;
mod webrtc_signal;

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

	/// Run a relay node for browser WebRTC peers.
	WebrtcSignal(webrtc_signal::Command),
}

pub async fn command(context: &CliContext, cli: &Cli, network_command: &Command) -> Result<ExitCode, anyhow::Error> {
	match &network_command.command {
		Commands::Listen(command) => listen::command(context, cli, network_command, command).await,
		Commands::WebrtcSignal(command) => webrtc_signal::command(context, cli, network_command, command).await,
	}
}
