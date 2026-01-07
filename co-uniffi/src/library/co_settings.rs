use crate::types::{level::CoLogLevel, network_settings::CoNetworkSettings};

#[derive(uniffi::Record, Debug, Default, Clone)]
pub struct CoSettings {
	pub identifier: String,
	pub path: Option<String>,
	pub network_settings: Option<CoNetworkSettings>,
	pub network: bool,
	pub no_keychain: bool,
	pub no_log: bool,
	pub log_level: Option<CoLogLevel>,
	pub no_default_features: bool,
	pub feature: Vec<String>,
}

#[uniffi::export]
pub fn co_settings_new(identifier: String) -> CoSettings {
	CoSettings { identifier, ..Default::default() }
}
