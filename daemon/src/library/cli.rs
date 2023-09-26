use std::{default, path::PathBuf};

/// Run COs via an HTTP Daemon.
#[derive(Debug, Clone, clap::Parser)]
pub struct Cli {
	/// Command.
	#[command(subcommand)]
	pub command: CliCommand,

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
	/// - data_path: <data_path>/data
	#[arg(long)]
	pub base_path: Option<PathBuf>,

	/// Storage folder path.
	#[arg(long)]
	pub storage_path: Option<PathBuf>,

	/// Configuration config path.
	#[arg(long)]
	pub config_path: Option<PathBuf>,

	/// Log path.
	#[arg(long)]
	pub log_path: Option<PathBuf>,

	/// Data storage path.
	#[arg(long)]
	pub data_path: Option<PathBuf>,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum CliCommand {
	/// Listen for HTTP connections.
	Http(HttpCommand),
}

#[derive(Debug, Clone, clap::Args)]
pub struct HttpCommand {
	/// The port to listen on for HTTP connections.
	#[arg(short, long)]
	pub port: Option<u16>,

	/// The port to listen on for peer-to-peer connections.
	#[arg(long)]
	pub p2p_port: Option<u16>,
}
