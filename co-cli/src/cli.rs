use std::path::PathBuf;

/// Run COs via an HTTP Daemon.
#[derive(Debug, Clone, clap::Parser)]
pub struct Cli {
	/// Command.
	#[command(subcommand)]
	pub command: CliCommand,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum CliCommand {
	/// Build the build-in cores.
	CoresBuild,
}
