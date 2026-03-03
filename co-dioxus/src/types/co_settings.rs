// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::library::cli::{Cli, CoLogLevel};
use cid::Cid;
use clap::Parser;
#[cfg(feature = "network")]
use co_sdk::NetworkSettings;
use co_sdk::{CoStorageSetting, Core, Cores, DynamicLocalSecret, GuardReference, Guards, LocalSecret};

#[derive(Debug, Clone, Default)]
pub struct CoSettings {
	pub identifier: String,
	pub storage: CoStorageSetting,
	#[cfg(feature = "network")]
	pub network_settings: NetworkSettings,
	#[cfg(feature = "network")]
	pub network: bool,
	pub no_keychain: bool,
	pub no_log: bool,
	pub log_level: CoLogLevel,
	pub no_default_features: bool,
	pub feature: Vec<String>,
	pub local_secret: Option<DynamicLocalSecret>,
	pub cores: Cores,
	pub guards: Guards,
}
impl CoSettings {
	pub fn new(identifier: &str) -> Self {
		CoSettings { identifier: identifier.into(), ..Default::default() }
	}

	/// Create `CoSettings` from command line args.
	pub fn cli(identifier: &str) -> Self {
		let mut cli = Cli::parse();
		if cli.instance_id.is_none() {
			cli.instance_id = Some(identifier.to_owned());
		}
		cli.into()
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

	pub fn with_core(mut self, core_cid: Cid, core: Core) -> Self {
		self.cores = self.cores.with_override(core_cid, core);
		self
	}

	pub fn with_guard(mut self, guard_cid: Cid, guard: GuardReference) -> Self {
		self.guards = self.guards.with_override(guard_cid, guard);
		self
	}
}
