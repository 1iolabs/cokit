// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

#[cfg(feature = "tracing")]
use co_tracing::LogArgs;
#[cfg(feature = "fs")]
use std::path::PathBuf;

/// Run COs via an HTTP Daemon.
#[derive(Debug, Clone, clap::Parser)]
#[non_exhaustive]
pub struct Cli {
	/// The instance ID of the process. Must be unique for every instance that runs in parallel.
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

	/// Logging configuration (`CO_LOG`, `-v`/`-q` — see `co-tracing`).
	#[cfg(feature = "tracing")]
	#[command(flatten)]
	pub log: LogArgs,

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

fn parse_bool(s: &str) -> Result<bool, String> {
	match s {
		"1" | "true" => Ok(true),
		"0" | "false" => Ok(false),
		_ => Err(format!("invalid bool: {s}")),
	}
}
