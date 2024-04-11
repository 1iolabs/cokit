use super::tags::TagValue;
use serde::{Deserialize, Serialize};
use std::{borrow::Borrow, fmt::Display, ops::Deref};

/// CO Unique ID.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CoId(String);
impl CoId {
	pub fn new(co: &str) -> Self {
		Self(co.to_owned())
	}

	pub fn as_str(&self) -> &str {
		&self.0
	}
}
impl From<String> for CoId {
	fn from(value: String) -> Self {
		Self(value)
	}
}
impl From<&str> for CoId {
	fn from(value: &str) -> Self {
		Self(value.to_owned())
	}
}
impl Into<String> for CoId {
	fn into(self) -> String {
		self.0
	}
}
impl Into<TagValue> for CoId {
	fn into(self) -> TagValue {
		self.0.into()
	}
}
impl AsRef<str> for CoId {
	fn as_ref(&self) -> &str {
		&self.0
	}
}
impl AsRef<CoId> for CoId {
	fn as_ref(&self) -> &CoId {
		&self
	}
}
impl Display for CoId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(&self.0)
	}
}
impl Borrow<str> for CoId {
	fn borrow(&self) -> &str {
		&self.0
	}
}
impl Deref for CoId {
	type Target = str;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
