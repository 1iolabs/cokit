// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::library::cli::Cli;
use cid::Cid;
use clap::Parser;
#[cfg(feature = "guard")]
use co_guard::{AccessGuard, DynamicAccessGuard, Guards};
#[cfg(feature = "guard")]
use co_sdk::GuardReference;
#[cfg(feature = "network")]
use co_sdk::NetworkSettings;
use co_sdk::{CoStorageSetting, ContactHandler, Core, Cores, DynamicContactHandler, DynamicLocalSecret, LocalSecret};
#[cfg(feature = "tracing")]
use co_tracing::{BoxedLayer, LogArgs};
#[cfg(feature = "tracing")]
use std::{
	fmt::{Formatter, Result},
	sync::Arc,
};

#[derive(Debug, Clone, Default)]
#[non_exhaustive]
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
	#[cfg(feature = "tracing")]
	pub log: LogArgs,
	/// App-provided tracing layer, applied when co-dioxus builds the subscriber.
	#[cfg(feature = "tracing")]
	pub(crate) log_layer: Option<LogLayer>,
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

	#[cfg(feature = "tracing")]
	pub fn with_log(self, logging: LogArgs) -> Self {
		Self { log: logging, ..self }
	}

	/// Attach a tracing layer, built lazily when co-dioxus creates the global subscriber.
	#[cfg(feature = "tracing")]
	pub fn with_log_layer(self, factory: impl Fn() -> BoxedLayer + Send + Sync + 'static) -> Self {
		Self { log_layer: Some(LogLayer(Arc::new(factory))), ..self }
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
			#[cfg(feature = "tracing")]
			log: cli.log,
			no_default_features: cli.no_default_features,
			feature: cli.feature,
			..Default::default()
		}
	}
}

/// A factory for an app-provided tracing layer, applied when co-dioxus builds the
/// global subscriber (see [`CoSettings::with_log_layer`]). Wrapped in `Arc` so
/// `CoSettings` stays `Clone`, with a manual `Debug` so it stays `Debug`.
#[cfg(feature = "tracing")]
#[derive(Clone)]
pub(crate) struct LogLayer(Arc<dyn Fn() -> BoxedLayer + Send + Sync>);
#[cfg(feature = "tracing")]
impl std::fmt::Debug for LogLayer {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result {
		f.write_str("LogLayer(..)")
	}
}
#[cfg(feature = "tracing")]
impl LogLayer {
	pub(crate) fn build(&self) -> BoxedLayer {
		(self.0)()
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
