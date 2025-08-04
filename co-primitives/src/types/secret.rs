use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};
use zeroize::Zeroize;

/// Simple wrapper type for secrents to not escape them.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Secret(#[serde(with = "serde_bytes")] Vec<u8>);
impl Secret {
	/// Create secret from vec.
	pub fn new(key: Vec<u8>) -> Self {
		Self(key)
	}

	/// Divulge (access) the secret.
	pub fn divulge(&self) -> &[u8] {
		&self.0
	}
}
impl Display for Secret {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("*****")
	}
}
impl Debug for Secret {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Secret").field("secret", &"*****").finish()
	}
}
impl Drop for Secret {
	fn drop(&mut self) {
		self.0.zeroize()
	}
}
impl From<Vec<u8>> for Secret {
	fn from(value: Vec<u8>) -> Self {
		Self(value)
	}
}
