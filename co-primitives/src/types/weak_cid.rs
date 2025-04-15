use cid::{serde::BytesToCidVisitor, Cid};
use serde::{Deserialize, Serialize};
use std::{borrow::Borrow, ops::Deref};

/// A CID that will be serialized as just bytes and will not be returned by [`crate::BlockLinks`].
#[derive(Debug, Clone, Copy, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct WeakCid(Cid);
impl WeakCid {
	pub fn new(cid: Cid) -> Self {
		Self(cid)
	}

	pub fn cid(&self) -> Cid {
		self.0
	}
}
impl From<Cid> for WeakCid {
	fn from(value: Cid) -> Self {
		Self(value)
	}
}
impl From<&Cid> for WeakCid {
	fn from(value: &Cid) -> Self {
		Self(*value)
	}
}
impl From<WeakCid> for Cid {
	fn from(value: WeakCid) -> Self {
		value.0
	}
}
impl<'de> Deserialize<'de> for WeakCid {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		Ok(Self(deserializer.deserialize_bytes(BytesToCidVisitor)?))
	}
}
impl Serialize for WeakCid {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let mut buf = [0u8; 128];
		let len = self
			.0
			.write_bytes(&mut buf[..])
			.expect("CID to serialize to bytes and fit into 128 bytes");
		serializer.serialize_bytes(&buf[0..len])
	}
}
impl AsRef<Cid> for WeakCid {
	fn as_ref(&self) -> &Cid {
		&self.0
	}
}
impl Borrow<Cid> for WeakCid {
	fn borrow(&self) -> &Cid {
		&self.0
	}
}
impl Borrow<Cid> for &WeakCid {
	fn borrow(&self) -> &Cid {
		&self.0
	}
}
impl Deref for WeakCid {
	type Target = Cid;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
