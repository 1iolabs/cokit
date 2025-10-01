use crate::CoSettings;
use clap::ValueEnum;
use co_sdk::NetworkSettings;
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
	#[arg(long, env = "CO_BASE_PATH")]
	pub base_path: Option<PathBuf>,

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
	#[arg(long, default_value_t = false, env = "CO_NO_NETWORK", value_parser = parse_bool)]
	pub no_network: bool,

	/// Force to generate new network peer id on startup.
	#[arg(long, default_value_t = false)]
	pub force_new_peer_id: bool,

	/// Disable default features.
	#[arg(long, default_value_t = false)]
	pub no_default_features: bool,

	/// Enable feature.
	#[arg(long, short = 'F')]
	pub feature: Vec<String>,
}
impl Into<CoSettings> for Cli {
	fn into(self) -> CoSettings {
		CoSettings {
			identifier: self.instance_id.unwrap_or_else(|| String::from("dioxus")),
			network: !self.no_network,
			network_settings: NetworkSettings::default().with_force_new_peer_id(self.force_new_peer_id),
			no_keychain: self.no_keychain,
			path: self.base_path,
			no_log: self.no_log,
			log_level: self.log_level,
			no_default_features: self.no_default_features,
			feature: self.feature,
			..Default::default()
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

#[derive(Debug, Default, Clone, ValueEnum)]
pub enum CoLogLevel {
	Error,
	Warn,
	#[default]
	Info,
	Debug,
	Trace,
}
impl Into<tracing::Level> for CoLogLevel {
	fn into(self) -> tracing::Level {
		match self {
			CoLogLevel::Error => tracing::Level::ERROR,
			CoLogLevel::Warn => tracing::Level::WARN,
			CoLogLevel::Info => tracing::Level::INFO,
			CoLogLevel::Debug => tracing::Level::DEBUG,
			CoLogLevel::Trace => tracing::Level::TRACE,
		}
	}
}
