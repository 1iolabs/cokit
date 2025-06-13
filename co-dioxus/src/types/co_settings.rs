use co_sdk::NetworkSettings;
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct CoSettings {
	pub identifier: String,
	pub path: Option<PathBuf>,
	pub network_settings: NetworkSettings,
	pub network: bool,
	pub no_keychain: bool,
	pub no_log: bool,
	pub no_default_features: bool,
	pub feature: Vec<String>,
}
impl CoSettings {
	pub fn new(identifier: &str) -> Self {
		CoSettings { identifier: identifier.into(), ..Default::default() }
	}

	pub fn with_path(self, path: &str) -> Self {
		Self { path: Some(path.into()), ..self }
	}

	pub fn with_network(self, network_settings: NetworkSettings) -> Self {
		Self { network: true, network_settings, ..self }
	}

	pub fn without_keychain(self) -> Self {
		Self { no_keychain: true, ..self }
	}
}
