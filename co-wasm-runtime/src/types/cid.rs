use super::{codec::MultiCodec, storage::Storage};
use libipld::Cid;
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, marker::PhantomData};

/// A typed link.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(into = "Cid", from = "Cid")]
pub struct Link<T: Clone> {
	_type: PhantomData<T>,
	cid: Cid,
}

impl<T: Clone> Link<T> {
	pub fn new(cid: Cid) -> Self {
		Self { cid, _type: Default::default() }
	}
}
impl<T> Link<T>
where
	T: Clone + serde::de::DeserializeOwned,
{
	pub fn resolve(&self, storage: &dyn Storage) -> Result<T, ResolveError> {
		match self.cid.codec().try_into().map_err(|v| ResolveError::UnknownCodec(v))? {
			MultiCodec::DagCbor => {
				let buf = storage.get(&self.cid);
				let result = serde_ipld_dagcbor::from_slice(buf.data()).map_err(|e| -> ResolveError { e.into() })?;
				Ok(result)
			},
			_ => Err(ResolveError::Codec),
		}
	}
}
impl<T: Clone> Into<Cid> for Link<T> {
	fn into(self) -> Cid {
		self.cid
	}
}
impl<T: Clone> From<Cid> for Link<T> {
	fn from(value: Cid) -> Self {
		Self::new(value)
	}
}
impl<T: Clone> AsRef<Cid> for Link<T> {
	fn as_ref(&self) -> &Cid {
		&self.cid
	}
}

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
	#[error("Unknown codec")]
	UnknownCodec(u64),
	#[error("Generic decoding error")]
	Codec,
	#[error("End of file")]
	Eof,
}

impl From<serde_ipld_dagcbor::DecodeError<Infallible>> for ResolveError {
	fn from(value: serde_ipld_dagcbor::DecodeError<Infallible>) -> Self {
		match value {
			serde_ipld_dagcbor::DecodeError::Msg(_) => ResolveError::Codec,
			serde_ipld_dagcbor::DecodeError::Read(_) => ResolveError::Codec,
			serde_ipld_dagcbor::DecodeError::Eof => ResolveError::Eof,
			serde_ipld_dagcbor::DecodeError::Mismatch { expect_major: _, byte: _ } => ResolveError::Codec,
			serde_ipld_dagcbor::DecodeError::TypeMismatch { name: _, byte: _ } => ResolveError::Codec,
			serde_ipld_dagcbor::DecodeError::CastOverflow(_) => ResolveError::Codec,
			serde_ipld_dagcbor::DecodeError::Overflow { name: _ } => ResolveError::Codec,
			serde_ipld_dagcbor::DecodeError::RequireBorrowed { name: _ } => ResolveError::Codec,
			serde_ipld_dagcbor::DecodeError::RequireLength { name: _, expect: _, value: _ } => ResolveError::Codec,
			serde_ipld_dagcbor::DecodeError::InvalidUtf8(_) => ResolveError::Codec,
			serde_ipld_dagcbor::DecodeError::Unsupported { byte: _ } => ResolveError::Codec,
			serde_ipld_dagcbor::DecodeError::DepthLimit => ResolveError::Codec,
			serde_ipld_dagcbor::DecodeError::TrailingData => ResolveError::Codec,
			serde_ipld_dagcbor::DecodeError::IndefiniteSize => ResolveError::Codec,
		}
	}
}
