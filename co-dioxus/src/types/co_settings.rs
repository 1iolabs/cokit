use crate::library::cli::{Cli, CoLogLevel};
use clap::Parser;
#[cfg(feature = "network")]
use co_sdk::NetworkSettings;
#[cfg(feature = "fs")]
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct CoSettings {
	pub identifier: String,
	#[cfg(feature = "fs")]
	pub path: Option<PathBuf>,
	#[cfg(feature = "network")]
	pub network_settings: NetworkSettings,
	#[cfg(feature = "network")]
	pub network: bool,
	pub no_keychain: bool,
	pub no_log: bool,
	pub log_level: CoLogLevel,
	pub no_default_features: bool,
	pub feature: Vec<String>,
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
		Self { path: Some(path.into()), ..self }
	}

	#[cfg(feature = "network")]
	pub fn with_network(self, network_settings: NetworkSettings) -> Self {
		Self { network: true, network_settings, ..self }
	}

	pub fn without_keychain(self) -> Self {
		Self { no_keychain: true, ..self }
	}
}
