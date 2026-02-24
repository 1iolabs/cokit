// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use std::path::PathBuf;

const APP_IDENTIFIER: &str = "co-http";

/// Run COs via an HTTP Daemon.
#[derive(Debug, Clone, clap::Parser)]
pub struct Cli {
	/// Command.
	#[command(subcommand)]
	pub command: CliCommand,

	/// The instance ID of the daemon. Must be uniqure for every instance that runs in parallel.
	#[arg(long, default_value_t = String::from(APP_IDENTIFIER))]
	pub instance_id: String,

	/// Extra multi-address(es) to dail.
	#[arg(short, long)]
	pub dail: Vec<String>,

	/// Force to generate a new peer-id on startup.
	///
	/// Warning: This will override the previous peer-id if it already exists.
	#[arg(long, default_value_t = false)]
	pub force_new_peer_id: bool,

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
	/// Listen for HTTP connections.
	Http(HttpCommand),
}

#[derive(Debug, Clone, clap::Args)]
pub struct HttpCommand {
	/// The port to listen on for HTTP connections.
	///
	/// Defaults to random OS assigned port.
	#[arg(short, long)]
	pub port: Option<u16>,

	/// The port to listen on for peer-to-peer connections.
	///
	/// Defaults to random OS assigned port.
	#[arg(long)]
	pub p2p_port: Option<u16>,
}
