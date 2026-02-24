// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::types::{level::CoLogLevel, network_settings::CoNetworkSettings};

#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
#[derive(Debug, Clone)]
pub struct CoSettings {
	pub identifier: String,
	pub path: Option<String>,
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
