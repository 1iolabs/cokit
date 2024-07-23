use libipld::Cid;
use std::fmt::Display;

/// MultiCodec matching utility.
///
/// See: https://github.com/multiformats/multicodec/blob/master/table.csv
#[derive(Debug, Copy, Clone, PartialEq)]
#[non_exhaustive]
#[repr(u64)]
pub enum MultiCodec {
	Identity = 0x0,
	Raw = 0x55,
	DagPb = 0x70,
	DagCbor = 0x71,
	CoEncryptedBlock = 0x301000,
	Unknown(u64),
}
impl MultiCodec {
	pub fn codec(codec: MultiCodec, cid: &Cid) -> Result<&Cid, MultiCodecError> {
		let actual_codec: MultiCodec = cid.codec().into();
		if actual_codec == codec {
			Ok(cid)
		} else {
			Err(MultiCodecError(*cid, codec, actual_codec))
		}
	}

	pub fn dag_cbor(cid: &Cid) -> Result<&Cid, MultiCodecError> {
		Self::codec(Self::DagCbor, cid)
	}
}
impl Display for MultiCodec {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Identity => write!(f, "Identity"),
			Self::Raw => write!(f, "Raw"),
			Self::DagPb => write!(f, "DagPb"),
			Self::DagCbor => write!(f, "DagCbor"),
			Self::CoEncryptedBlock => write!(f, "CoEncryptedBlock"),
			Self::Unknown(c) => write!(f, "{:#x}", c),
		}
	}
}
impl From<MultiCodec> for u64 {
	fn from(val: MultiCodec) -> Self {
		match val {
			MultiCodec::Identity => 0x0,
			MultiCodec::Raw => 0x55,
			MultiCodec::DagPb => 0x70,
			MultiCodec::DagCbor => 0x71,
			MultiCodec::CoEncryptedBlock => 0x301000,
			MultiCodec::Unknown(i) => i,
		}
	}
}
impl From<u64> for MultiCodec {
	fn from(value: u64) -> MultiCodec {
		match value {
			// known
			0x0 => MultiCodec::Identity,
			0x55 => MultiCodec::Raw,
			0x70 => MultiCodec::DagPb,
			0x71 => MultiCodec::DagCbor,
			0x301000 => MultiCodec::CoEncryptedBlock,

			// unknown
			value => MultiCodec::Unknown(value),
		}
	}
}
impl From<&Cid> for MultiCodec {
	fn from(value: &Cid) -> MultiCodec {
		Self::from(value.codec())
	}
}

#[derive(Debug, thiserror::Error)]
#[error("Expected {0} codec to be {1:?} got {2:?}")]
pub struct MultiCodecError(Cid, MultiCodec, MultiCodec);
