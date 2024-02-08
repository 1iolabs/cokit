use crate::commands::{cbor, co};
use std::path::PathBuf;

pub const APP_IDENTIFIER: &str = "co-cli";

/// Run COs via an HTTP Daemon.
#[derive(Debug, Clone, clap::Parser)]
pub struct Cli {
	/// Command.
	#[command(subcommand)]
	pub command: CliCommand,

	/// Base path.
	///
	/// If this option ispecified all files are stored in this path (if not explicitly overwritten):
	/// - storage_path: <base_path>/storage
	/// - config_path: <base_path>/etc
	/// - log_path: <base_path>/log
	///
	/// Default: `~/Application Support/co.app.1io.co`
	#[arg(long)]
	pub base_path: Option<PathBuf>,

	/// Log path.
	#[arg(long)]
	pub log_path: Option<PathBuf>,

	/// Disable logging to file.
	#[arg(long, default_value_t = false)]
	pub no_log: bool,

	/// Read/Write Local CO encryption key to file instead of the OS keychain.
	///
	/// Warning: This option is INSECURE only use when you know the implications.
	#[arg(long, default_value_t = false)]
	pub no_keychain: bool,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum CliCommand {
	/// DAG-CBOR Utilities.
	Co(co::Command),

	/// Build the build-in cores.
	CoreBuildBuiltin,

	/// DAG-CBOR Utilities.
	Cbor(cbor::Command),
}
