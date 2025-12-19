use cid::Cid;
use core::fmt::Debug;
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::fmt::Display;

/// Known Muiticodecs
/// See: https://github.com/multiformats/multicodec/blob/master/table.csv
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize_repr, Deserialize_repr)]
#[non_exhaustive]
#[repr(u64)]
pub enum KnownMultiCodec {
	/// Tag: multihash
	/// Status: permanent
	Identity = 0x0,

	/// Tag: multihash
	/// Status: permanent
	Sha1 = 0x11,
	/// Tag: multihash
	/// Status: permanent
	Sha2256 = 0x12,
	/// Tag: multihash
	/// Status: permanent
	Sha2512 = 0x13,
	/// Tag: multihash
	/// Status: permanent
	Sha3512 = 0x14,
	/// Tag: multihash
	/// Status: permanent
	Sha3384 = 0x15,
	/// Tag: multihash
	/// Status: permanent
	Sha3256 = 0x16,
	/// Tag: multihash
	/// Status: permanent
	Sha3224 = 0x17,
	/// Tag: multihash
	/// Status: draft
	Shake128 = 0x18,
	/// Tag: multihash
	/// Status: draft
	Shake256 = 0x19,
	/// keccak has variable output length. The number specifies the core length
	/// Tag: multihash
	/// Status: draft
	Keccak224 = 0x1a,
	/// Tag: multihash
	/// Status: draft
	Keccak256 = 0x1b,
	/// Tag: multihash
	/// Status: draft
	Keccak384 = 0x1c,
	/// Tag: multihash
	/// Status: draft
	Keccak512 = 0x1d,
	/// BLAKE3 has a default 32 byte output length. The maximum length is (2^64)-1 bytes.
	/// Tag: multihash
	/// Status: draft
	Blake3 = 0x1e,
	/// aka SHA-384; as specified by FIPS 180-4.
	/// Tag: multihash
	/// Status: permanent
	Sha2384 = 0x20,

	/// raw binary
	/// Tag: ipld
	/// Status: permanent
	Raw = 0x55,

	/// MerkleDAG protobuf
	/// Tag: ipld
	/// Status: permanent
	DagPb = 0x70,
	/// MerkleDAG cbor
	/// Tag: ipld
	/// Status: permanent
	DagCbor = 0x71,

	/// MerkleDAG json
	/// Tag: ipld
	/// Status: permanent
	DagJson = 0x0129,

	/// Co Encrypted Block wrapped in [`KnownMultiCodec::DagCbor`].
	CoEncryptedBlock = 0x301000,

	/// [`crate::CoReference`] as [`KnownMultiCodec::DagCbor`].
	CoReference = 0x301001,
}
impl KnownMultiCodec {
	pub fn multi_codec(&self) -> MultiCodec {
		MultiCodec::Known(*self)
	}
}
impl TryFrom<u64> for KnownMultiCodec {
	type Error = u64;

	fn try_from(value: u64) -> Result<Self, Self::Error> {
		serde_json::from_value(value.into()).map_err(|_| value)
	}
}
impl From<KnownMultiCodec> for u64 {
	fn from(value: KnownMultiCodec) -> Self {
		value as u64
	}
}
impl PartialEq<u64> for KnownMultiCodec {
	fn eq(&self, other: &u64) -> bool {
		(*self) as u64 == *other
	}
}

/// MultiCodec matching utility.
///
/// See: https://github.com/multiformats/multicodec/blob/master/table.csv
#[derive(Copy, Clone)]
#[non_exhaustive]
#[repr(u64)]
pub enum MultiCodec {
	Known(KnownMultiCodec),
	Unknown(u64),
}
impl Ord for MultiCodec {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.codec().cmp(&other.codec())
	}
}
impl PartialOrd for MultiCodec {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}
impl Eq for MultiCodec {}
impl PartialEq for MultiCodec {
	fn eq(&self, other: &Self) -> bool {
		self.codec() == other.codec()
	}
}
impl PartialEq<KnownMultiCodec> for MultiCodec {
	fn eq(&self, other: &KnownMultiCodec) -> bool {
		self == &other.multi_codec()
	}
}
impl PartialEq<u64> for MultiCodec {
	fn eq(&self, other: &u64) -> bool {
		&self.codec() == other
	}
}
impl<'de> serde::Deserialize<'de> for MultiCodec {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		Ok(MultiCodec::from(u64::deserialize(deserializer)?))
	}
}
impl serde::Serialize for MultiCodec {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serializer.serialize_u64((*self).into())
	}
}
impl MultiCodec {
	/// Expect cid to be of type codec.
	pub fn with_codec(codec: impl Into<MultiCodec>, cid: &Cid) -> Result<&Cid, MultiCodecError> {
		let codec = codec.into();
		let actual_codec: MultiCodec = cid.codec().into();
		if actual_codec == codec {
			Ok(cid)
		} else {
			Err(MultiCodecError(*cid, codec, actual_codec))
		}
	}

	/// Expect cid to be of type codec.
	pub fn is_any_codec<'a, C: Into<MultiCodec>>(codecs: impl IntoIterator<Item = C>, cid: &Cid) -> Option<&Cid> {
		let actual_codec: MultiCodec = cid.codec().into();
		if codecs.into_iter().map(Into::into).any(|c| c == actual_codec) {
			Some(cid)
		} else {
			None
		}
	}

	/// Error if not DAG-CBOR or a codec that is represented in DAG-CBOR.
	pub fn with_cbor(cid: &Cid) -> Result<&Cid, MultiCodecError> {
		Self::is_any_codec([KnownMultiCodec::DagCbor, KnownMultiCodec::CoReference], cid)
			.ok_or_else(|| MultiCodecError(*cid, MultiCodec::Known(KnownMultiCodec::DagCbor), MultiCodec::from(cid)))
	}

	/// Is DAG-CBOR or a codec that is represented in DAG-CBOR.
	pub fn is_cbor(cid: impl Into<MultiCodec>) -> bool {
		match cid.into() {
			MultiCodec::Known(KnownMultiCodec::DagCbor) | MultiCodec::Known(KnownMultiCodec::CoReference) => true,
			_ => false,
		}
	}

	pub fn is(actual: impl Into<MultiCodec>, expect: impl Into<MultiCodec>) -> bool {
		actual.into() == expect.into()
	}

	pub fn codec(&self) -> u64 {
		(*self).into()
	}
}
impl Display for MultiCodec {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Known(m) => write!(f, "{:?}", m),
			Self::Unknown(c) => write!(f, "{:#x}", c),
		}
	}
}
impl Debug for MultiCodec {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Known(m) => write!(f, "{:?} ({:#x})", m, self.codec()),
			Self::Unknown(c) => write!(f, "{:#x}", c),
		}
	}
}
impl From<KnownMultiCodec> for MultiCodec {
	fn from(value: KnownMultiCodec) -> Self {
		MultiCodec::Known(value)
	}
}
impl From<MultiCodec> for u64 {
	fn from(val: MultiCodec) -> Self {
		match val {
			MultiCodec::Known(v) => v.into(),
			MultiCodec::Unknown(v) => v,
		}
	}
}
impl From<u64> for MultiCodec {
	fn from(value: u64) -> MultiCodec {
		match KnownMultiCodec::try_from(value) {
			Ok(v) => MultiCodec::Known(v),
			Err(v) => MultiCodec::Unknown(v),
		}
	}
}
impl From<&Cid> for MultiCodec {
	fn from(value: &Cid) -> MultiCodec {
		Self::from(value.codec())
	}
}
impl From<Cid> for MultiCodec {
	fn from(value: Cid) -> Self {
		Self::from(value.codec())
	}
}

#[derive(Debug, thiserror::Error)]
#[error("Expected {0} codec to be {1} got {2}")]
pub struct MultiCodecError(Cid, MultiCodec, MultiCodec);

#[cfg(test)]
mod tests {
	use super::MultiCodec;
	use crate::{BlockSerializer, CoReference, DefaultParams, KnownMultiCodec};
	use serde::{Deserialize, Serialize};

	#[test]
	fn test_eq() {
		assert_eq!(MultiCodec::Known(KnownMultiCodec::DagCbor), MultiCodec::Known(KnownMultiCodec::DagCbor));
		assert_eq!(MultiCodec::Unknown(0x71u64), MultiCodec::Known(KnownMultiCodec::DagCbor));
		assert_ne!(MultiCodec::Unknown(0x72u64), MultiCodec::Known(KnownMultiCodec::DagCbor));
	}

	#[test]
	fn test_from_u64_known() {
		assert!(matches!(MultiCodec::from(0x71u64), MultiCodec::Known(KnownMultiCodec::DagCbor)));
	}

	#[test]
	fn test_from_u64_unknown() {
		assert!(matches!(MultiCodec::from(0xdeadbeefu64), MultiCodec::Unknown(0xdeadbeefu64)));
	}

	#[test]
	fn test_into_u64_known() {
		assert_eq!(MultiCodec::from(KnownMultiCodec::DagCbor).codec(), 0x71u64);
	}

	#[test]
	fn test_into_u64_unknown() {
		assert_eq!(MultiCodec::from(0xdeadbeefu64).codec(), 0xdeadbeefu64);
	}

	#[test]
	fn test_serde() {
		#[derive(Debug, PartialEq, Serialize, Deserialize)]
		struct Test {
			codec: MultiCodec,
		}

		// serialize
		assert_eq!(serde_json::to_string(&Test { codec: KnownMultiCodec::DagCbor.into() }).unwrap(), "{\"codec\":113}");
		assert_eq!(serde_json::to_string(&Test { codec: 0xdeadbeefu64.into() }).unwrap(), "{\"codec\":3735928559}");

		// deserialize
		assert_eq!(
			serde_json::from_str::<Test>("{\"codec\":113}").unwrap(),
			Test { codec: MultiCodec::Known(KnownMultiCodec::DagCbor) }
		);
		assert_eq!(
			serde_json::from_str::<Test>("{\"codec\":3735928559}").unwrap(),
			Test { codec: MultiCodec::Unknown(0xdeadbeefu64) }
		);
	}

	#[test]
	fn test_cid() {
		let block = BlockSerializer::<DefaultParams>::new_codec(KnownMultiCodec::CoReference)
			.serialize(&CoReference::Weak(1))
			.unwrap();
		assert_eq!(block.cid().to_string(), "baga2bqabdyqe2tf374ji3ixvay5hqmwyymxxpgjtxmqfijehizup5f5pypp6bda");
	}
}
