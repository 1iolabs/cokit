// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::library::cli::{Cli, CoLogLevel};
use cid::Cid;
use clap::Parser;
#[cfg(feature = "guard")]
use co_guard::{AccessGuard, DynamicAccessGuard, Guards};
#[cfg(feature = "guard")]
use co_sdk::GuardReference;
#[cfg(feature = "network")]
use co_sdk::NetworkSettings;
use co_sdk::{CoStorageSetting, ContactHandler, Core, Cores, DynamicContactHandler, DynamicLocalSecret, LocalSecret};

#[derive(Debug, Clone, Default)]
pub struct CoSettings {
	/// Application Bundle Identifier.
	///
	/// Example: `com.1io.my-todo-app`
	pub bundle_identifier: String,
	/// Instance identifier.
	///
	/// To support to read/write the Local CO from multiple processes.
	/// Never give two application instances on the same device the same instance identifier.
	///
	/// Example: `my-todo-app`
	pub identifier: String,
	pub storage: CoStorageSetting,
	#[cfg(feature = "network")]
	pub network_settings: NetworkSettings,
	#[cfg(feature = "network")]
	pub network: bool,
	pub no_keychain: bool,
	pub log: CoLog,
	pub log_level: CoLogLevel,
	pub no_default_features: bool,
	pub feature: Vec<String>,
	pub local_secret: Option<DynamicLocalSecret>,
	#[cfg(feature = "guard")]
	pub access_guard: Option<DynamicAccessGuard>,
	pub contact_handler: Option<DynamicContactHandler>,
	pub cores: Cores,
	#[cfg(feature = "guard")]
	pub guards: Guards,
}
impl CoSettings {
	pub fn new(bundle_identifier: &str, identifier: &str) -> Self {
		CoSettings { bundle_identifier: bundle_identifier.into(), identifier: identifier.into(), ..Default::default() }
	}

	/// Create `CoSettings` from command line args.
	pub fn cli(bundle_identifier: &str, identifier: &str) -> Self {
		let mut cli = Cli::parse();
		if cli.instance_id.is_none() {
			cli.instance_id = Some(identifier.to_owned());
		}
		Self::from_cli(bundle_identifier.into(), cli)
	}

	pub fn with_log(self, log: CoLog) -> Self {
		Self { log, ..self }
	}

	pub fn with_log_level(self, log_level: impl Into<CoLogLevel>) -> Self {
		Self { log_level: log_level.into(), ..self }
	}

	#[cfg(feature = "fs")]
	pub fn with_path(self, path: &str) -> Self {
		Self { storage: CoStorageSetting::Path(path.into()), ..self }
	}

	pub fn with_memory(self) -> Self {
		Self { storage: CoStorageSetting::Memory, ..self }
	}

	#[cfg(all(feature = "indexeddb", target_arch = "wasm32"))]
	pub fn with_indexeddb(self, secret: impl LocalSecret + 'static) -> Self {
		Self { storage: CoStorageSetting::IndexedDb, local_secret: Some(DynamicLocalSecret::new(secret)), ..self }
	}

	#[cfg(feature = "network")]
	pub fn with_network(self, network_settings: NetworkSettings) -> Self {
		Self { network: true, network_settings, ..self }
	}

	pub fn without_keychain(self) -> Self {
		Self { no_keychain: true, ..self }
	}

	pub fn with_local_secret(self, secret: impl LocalSecret + 'static) -> Self {
		Self { local_secret: Some(DynamicLocalSecret::new(secret)), ..self }
	}

	#[cfg(feature = "guard")]
	pub fn with_access_guard(self, guard: impl AccessGuard + 'static) -> Self {
		Self { access_guard: Some(DynamicAccessGuard::new(guard)), ..self }
	}

	pub fn with_contact_handler(self, handler: impl ContactHandler + 'static) -> Self {
		Self { contact_handler: Some(DynamicContactHandler::new(handler)), ..self }
	}

	pub fn with_core(mut self, core_cid: Cid, core: Core) -> Self {
		self.cores = self.cores.with_override(core_cid, core);
		self
	}

	#[cfg(feature = "guard")]
	pub fn with_guard(mut self, guard_cid: Cid, guard: GuardReference) -> Self {
		self.guards = self.guards.with_override(guard_cid, guard);
		self
	}

	pub fn from_cli(bundle_identifier: String, cli: Cli) -> CoSettings {
		CoSettings {
			bundle_identifier,
			storage: co_storage(&cli),
			identifier: cli.instance_id.unwrap_or_else(|| String::from("dioxus")),
			#[cfg(feature = "network")]
			network: !cli.no_network,
			#[cfg(feature = "network")]
			network_settings: NetworkSettings::default().with_force_new_peer_id(cli.force_new_peer_id),
			no_keychain: cli.no_keychain,
			log: if cli.no_log { CoLog::None } else { CoLog::Default },
			log_level: cli.log_level,
			no_default_features: cli.no_default_features,
			feature: cli.feature,
			..Default::default()
		}
	}
}

#[derive(Debug, Clone, Default)]
pub enum CoLog {
	/// No (COKIT managed) logging.
	#[default]
	None,

	/// Use default logging for the platform using identifier.
	Default,

	/// Print logs to stderr.
	#[cfg(feature = "tracing")]
	Print,

	/// Print logs to browser console.
	#[cfg(feature = "web")]
	Console,

	/// Write logs to file in bunyan format.
	/// If no path is specified `$CO_BASE_PATH/log/co.log` is used.
	#[cfg(all(feature = "fs", feature = "tracing"))]
	File(Option<std::path::PathBuf>),

	/// Send logs to OS logger (Console).
	#[cfg(feature = "tracing-oslog")]
	Os,
}
impl CoLog {
	/// Resolve default to platform specific logger.
	#[allow(unreachable_code)]
	pub fn with_resolved_default(self) -> Self {
		if let Self::Default = self {
			// web
			#[cfg(feature = "web")]
			return Self::Console;

			// mobile
			#[cfg(all(feature = "mobile", feature = "tracing-oslog"))]
			return Self::Os;

			// tracing
			#[cfg(all(feature = "desktop", feature = "fs", feature = "tracing"))]
			return Self::File(None);

			// none
			Self::None
		} else {
			self
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
