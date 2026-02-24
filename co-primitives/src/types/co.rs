// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use super::tags::TagValue;
use serde::{Deserialize, Serialize};
use std::{borrow::Borrow, fmt::Display, ops::Deref};

/// CO Unique ID.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
pub struct CoId(String);
impl CoId {
	pub fn new(co: impl Into<String>) -> Self {
		Self(co.into())
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
impl From<&String> for CoId {
	fn from(value: &String) -> Self {
		Self(value.to_owned())
	}
}
impl From<&CoId> for CoId {
	fn from(value: &CoId) -> Self {
		value.to_owned()
	}
}
impl From<CoId> for String {
	fn from(val: CoId) -> Self {
		val.0
	}
}
impl From<CoId> for TagValue {
	fn from(val: CoId) -> Self {
		val.0.into()
	}
}
impl AsRef<str> for CoId {
	fn as_ref(&self) -> &str {
		&self.0
	}
}
impl AsRef<CoId> for CoId {
	fn as_ref(&self) -> &CoId {
		self
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
