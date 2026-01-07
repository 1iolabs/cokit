use std::sync::Arc;

#[derive(Debug, thiserror::Error, uniffi::Object)]
#[error("{source:?}")]
pub struct CoError {
	#[source]
	source: anyhow::Error,
}
#[uniffi::export]
impl CoError {
	fn message(&self) -> String {
		self.to_string()
	}
}
impl CoError {
	pub fn new<E: Into<anyhow::Error>>(err: E) -> Self {
		Self { source: err.into() }
	}

	pub fn new_arc<E: Into<anyhow::Error>>(err: E) -> Arc<Self> {
		Arc::new(Self { source: err.into() })
	}
}
