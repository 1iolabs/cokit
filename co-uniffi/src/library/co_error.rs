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
