// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::CoSettings;
use clap::ValueEnum;
use co_sdk::CoStorageSetting;
#[cfg(feature = "network")]
use co_sdk::NetworkSettings;
#[cfg(feature = "fs")]
use std::path::PathBuf;

/// Run COs via an HTTP Daemon.
#[derive(Debug, Clone, clap::Parser)]
#[non_exhaustive]
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
	/// Default: `~/Application Support/co.app.1io.co`
	/// Env: CO_BASE_PATH
	#[cfg(feature = "fs")]
	#[arg(long, env = "CO_BASE_PATH")]
	pub base_path: Option<PathBuf>,

	/// Start instance in memory.
	/// Implies: no_keychain, no_log
	#[arg(long, env = "CO_MEMORY")]
	pub memory: bool,

	/// Disable logging to file.
	#[arg(long, default_value_t = false)]
	pub no_log: bool,

	/// Only log level and above.
	#[arg(long, value_enum, default_value_t, env = "CO_LOG_LEVEL")]
	pub log_level: CoLogLevel,

	/// Read/Write Local CO encryption key to file instead of the OS keychain.
	///
	/// Warning: This option is INSECURE only use when you know the implications.
	/// Env: CO_NO_KEYCHAIN
	#[arg(long, default_value_t = false, env = "CO_NO_KEYCHAIN", value_parser = parse_bool)]
	pub no_keychain: bool,

	/// Skip networking.
	/// Env: CO_NO_NETWORK
	#[cfg(feature = "network")]
	#[arg(long, default_value_t = false, env = "CO_NO_NETWORK", value_parser = parse_bool)]
	pub no_network: bool,

	/// Force to generate new network peer id on startup.
	#[cfg(feature = "network")]
	#[arg(long, default_value_t = false)]
	pub force_new_peer_id: bool,

	/// Disable default features.
	#[arg(long, default_value_t = false)]
	pub no_default_features: bool,

	/// Enable feature.
	#[arg(long, short = 'F')]
	pub feature: Vec<String>,
}
impl From<Cli> for CoSettings {
	fn from(cli: Cli) -> Self {
		CoSettings {
			storage: co_storage(&cli),
			identifier: cli.instance_id.unwrap_or_else(|| String::from("dioxus")),
			#[cfg(feature = "network")]
			network: !cli.no_network,
			#[cfg(feature = "network")]
			network_settings: NetworkSettings::default().with_force_new_peer_id(cli.force_new_peer_id),
			no_keychain: cli.no_keychain,
			no_log: cli.no_log,
			log_level: cli.log_level,
			no_default_features: cli.no_default_features,
			feature: cli.feature,
			local_secret: None,
		}
	}
}

fn co_storage(_cli: &Cli) -> CoStorageSetting {
	#[cfg(feature = "fs")]
	if !_cli.memory {
		return match _cli.base_path.clone() {
			Some(path) => CoStorageSetting::Path(path),
			None => CoStorageSetting::PathDefault,
		};
	}
	CoStorageSetting::Memory
}

fn parse_bool(s: &str) -> Result<bool, String> {
	match s {
		"1" | "true" => Ok(true),
		"0" | "false" => Ok(false),
		_ => Err(format!("invalid bool: {s}")),
	}
}

#[derive(Debug, Default, Clone, Copy, ValueEnum)]
pub enum CoLogLevel {
	Error,
	Warn,
	#[default]
	Info,
	Debug,
	Trace,
}
impl From<CoLogLevel> for tracing::Level {
	fn from(value: CoLogLevel) -> Self {
		match value {
			CoLogLevel::Error => tracing::Level::ERROR,
			CoLogLevel::Warn => tracing::Level::WARN,
			CoLogLevel::Info => tracing::Level::INFO,
			CoLogLevel::Debug => tracing::Level::DEBUG,
			CoLogLevel::Trace => tracing::Level::TRACE,
		}
	}
}
