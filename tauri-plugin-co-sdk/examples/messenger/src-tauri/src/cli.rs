use std::path::PathBuf;
use tauri_plugin_co_sdk::library::co_application::CoApplicationSettings;

const APP_IDENTIFIER: &str = "coapp-messenger-demo";

/// Run COs via an HTTP Daemon.
#[derive(Debug, Clone, clap::Parser)]
pub struct Cli {
	/// The instance ID of the daemon. Must be uniqure for every instance that runs in parallel.
	/// Env: CO_INSTANCE_ID
	#[arg(long, default_value_t = String::from(APP_IDENTIFIER))]
	pub instance_id: String,

	/// Base path.
	///
	/// If this option ispecified all files are stored in this path (if not explicitly overwritten):
	/// - storage_path: <base_path>/storage
	/// - config_path: <base_path>/etc
	/// - log_path: <base_path>/log
	///
	/// Default: `~/Library/Application Support/co.app.1io.co`
	/// Env: CO_BASE_PATH
	#[arg(long)]
	pub base_path: Option<PathBuf>,

	/// Disable logging to file.
	#[arg(long, default_value_t = false)]
	pub no_log: bool,

	/// Read/Write Local CO encryption key to file instead of the OS keychain.
	///
	/// Warning: This option is INSECURE only use when you know the implications.
	/// Env: CO_NO_KEYCHAIN
	#[arg(long, default_value_t = false)]
	pub no_keychain: bool,

	/// Skip networking.
	/// Env: CO_NO_NETWORK
	#[arg(long, default_value_t = false)]
	pub no_network: bool,

	/// Force to generate new network peer id on startup.
	#[arg(long, default_value_t = false)]
	pub force_new_peer_id: bool,

	/// Sets the local CO to automatically accept incoming invites
	#[arg(long, default_value_t = false)]
	pub auto_accept_invite: bool,
}
impl Cli {
	/// Use environment variables to override values.
	pub fn with_env(self) -> Self {
		let instance_id = std::env::var("CO_INSTANCE_ID").unwrap_or(self.instance_id);
		let base_path: Option<PathBuf> = match std::env::var("CO_BASE_PATH") {
			Ok(path) => Some(path.into()),
			Err(_) => self.base_path,
		};
		let no_keychain = std::env::var("CO_NO_KEYCHAIN").is_ok() || self.no_keychain;
		let no_network = std::env::var("CO_NO_NETWORK").is_ok() || self.no_network;
		Self { instance_id, base_path, no_keychain, no_network, ..self }
	}
}
impl Into<CoApplicationSettings> for Cli {
	fn into(self) -> CoApplicationSettings {
		CoApplicationSettings {
			instance_id: self.instance_id,
			network: !self.no_network,
			force_new_peer_id: self.force_new_peer_id,
			no_keychain: self.no_keychain,
			base_path: self.base_path,
			no_log: self.no_log,
			auto_accept_invite: self.auto_accept_invite,
			..Default::default()
		}
	}
}
