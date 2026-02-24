// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::library::co_application::CoApplicationSettings;
use std::path::PathBuf;

/// Run COs via an HTTP Daemon.
#[derive(Debug, Clone, clap::Parser)]
pub struct Cli {
	/// The instance ID of the daemon. Must be uniqure for every instance that runs in parallel.
	/// Env: CO_INSTANCE_ID
	#[arg(long, env = "CO_INSTANCE_ID")]
	pub instance_id: Option<String>,

	/// Base path.
	///
	/// If this option ispecified all files are stored in this path (if not explicitly overwritten):
	/// - storage_path: <base_path>/storage
	/// - config_path: <base_path>/etc
	/// - log_path: <base_path>/log
	///
	/// Default: `~/Library/Application Support/co.app.1io.co`
	/// Env: CO_BASE_PATH
	#[arg(long, env = "CO_BASE_PATH")]
	pub base_path: Option<PathBuf>,

	/// Disable logging to file.
	#[arg(long, default_value_t = false)]
	pub no_log: bool,

	/// Read/Write Local CO encryption key to file instead of the OS keychain.
	///
	/// Warning: This option is INSECURE only use when you know the implications.
	/// Env: CO_NO_KEYCHAIN
	#[arg(long, default_value_t = false, env = "CO_NO_KEYCHAIN", value_parser = parse_bool)]
	pub no_keychain: bool,

	/// Skip networking.
	/// Env: CO_NO_NETWORK
	#[arg(long, default_value_t = false, env = "CO_NO_NETWORK", value_parser = parse_bool)]
	pub no_network: bool,

	/// Force to generate new network peer id on startup.
	#[arg(long, default_value_t = false)]
	pub force_new_peer_id: bool,
}
impl From<Cli> for CoApplicationSettings {
	fn from(value: Cli) -> Self {
		CoApplicationSettings {
			instance_id: value.instance_id.unwrap_or_else(|| String::from("tauri")),
			network: !value.no_network,
			force_new_peer_id: value.force_new_peer_id,
			no_keychain: value.no_keychain,
			base_path: value.base_path,
			no_log: value.no_log,
		}
	}
}

fn parse_bool(s: &str) -> Result<bool, String> {
	match s {
		"1" | "true" => Ok(true),
		"0" | "false" => Ok(false),
		_ => Err(format!("invalid bool: {s}")),
	}
}
