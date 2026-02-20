use crate::types::level::CoLogLevel;
#[cfg(feature = "network")]
use crate::types::network_settings::CoNetworkSettings;

#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
#[derive(Debug, Clone)]
pub struct CoSettings {
	pub identifier: String,
	pub path: Option<String>,
	#[cfg(feature = "network")]
	pub network_settings: Option<CoNetworkSettings>,
	pub network: Option<bool>,
	pub no_keychain: Option<bool>,
	pub no_log: Option<bool>,
	pub log_level: Option<CoLogLevel>,
	pub no_default_features: Option<bool>,
	pub feature: Option<Vec<String>>,
}
impl Default for CoSettings {
	fn default() -> Self {
		Self {
			identifier: Default::default(),
			path: Default::default(),
			#[cfg(feature = "network")]
			network_settings: Default::default(),
			network: Some(true),
			no_keychain: Some(true),
			no_log: Default::default(),
			log_level: Default::default(),
			no_default_features: Default::default(),
			feature: Default::default(),
		}
	}
}

#[cfg(feature = "uniffi")]
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn co_settings_new(identifier: String) -> CoSettings {
	CoSettings { identifier, ..Default::default() }
}
