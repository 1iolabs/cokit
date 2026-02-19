// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
#[derive(Debug, thiserror::Error)]
#[error("{source:?}")]
pub struct CoError {
	#[from]
	source: anyhow::Error,
}
#[cfg_attr(feature = "uniffi", uniffi::export)]
impl CoError {
	#[cfg_attr(feature = "frb", flutter_rust_bridge::frb(sync))]
	pub fn message(&self) -> String {
		self.to_string()
	}
}
impl CoError {
	pub fn new<E: Into<anyhow::Error>>(err: E) -> Self {
		Self { source: err.into() }
	}
}
