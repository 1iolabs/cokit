use std::convert::Infallible;

use crate::{Block, Storage};
use co_primitives::Link;
use libipld::{
	cbor::DagCborCodec,
	multihash::{Code, MultihashDigest},
	Cid,
};

pub trait StorageExt: Storage {
	/// Get value from link.
	fn get_value<T>(&self, link: &Link<T>) -> Result<T, ResolveError>
	where
		T: Clone + serde::de::DeserializeOwned,
	{
		match link.cid().codec() {
			v if v == Into::<u64>::into(DagCborCodec) => {
				let buf = self.get(link.cid());
				let result = serde_ipld_dagcbor::from_slice(buf.data())?;
				Ok(result)
			},
			v => Err(ResolveError::UnknownCodec(v)),
		}
	}

	/// Create link for value.
	fn set_value<T>(&mut self, value: &T) -> Link<T>
	where
		T: Clone + serde::Serialize,
	{
		let data = serde_ipld_dagcbor::to_vec(value).expect("value to serialize");
		let mh = Code::Blake3_256.digest(&data);
		let cid = Cid::new_v1(DagCborCodec.into(), mh);
		let result = Link::new(cid.clone());
		let block = Block::new_unchecked(cid, data);
		self.set(block);
		result
	}
}
impl<T> StorageExt for T where T: Storage + ?Sized {}

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
	#[error("Unknown codec")]
	UnknownCodec(u64),
	#[error("Generic decoding error")]
	Codec,
	#[error("Invalid argument")]
	InvalidArgument,
}
impl From<serde_ipld_dagcbor::DecodeError<Infallible>> for ResolveError {
	fn from(value: serde_ipld_dagcbor::DecodeError<Infallible>) -> Self {
		match value {
			serde_ipld_dagcbor::DecodeError::Msg(_) => ResolveError::Codec,
			serde_ipld_dagcbor::DecodeError::Read(_) => ResolveError::Codec,
			serde_ipld_dagcbor::DecodeError::Eof => ResolveError::InvalidArgument,
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
