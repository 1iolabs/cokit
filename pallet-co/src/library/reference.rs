extern crate alloc;

use crate::{
	error::{Error, Result},
	std::*,
};
use alloc::{collections::BTreeMap, string::String};
use cid::Cid;
use libipld_core::ipld::Ipld;

#[derive(Debug, Clone, PartialEq)]
pub struct ListReference {
	/// The referenced items.
	pub version: ListReferenceVersion,

	/// The referenced items.
	pub reference: Cid,

	/// Linked list to next `ListReference`.
	pub next: Option<Cid>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub enum ListReferenceVersion {
	#[default]
	V1 = 1,
}

impl Into<Ipld> for ListReference {
	fn into(self) -> Ipld {
		let mut data = BTreeMap::<String, Ipld>::new();
		data.insert(String::from("v"), Ipld::Integer(self.version as i128));
		data.insert(String::from("r"), Ipld::Link(self.reference));
		if let Some(next) = self.next {
			data.insert(String::from("n"), Ipld::Link(next));
		}
		Ipld::Map(data)
	}
}

#[cfg(feature = "std")]
impl TryInto<Vec<u8>> for ListReference {
	type Error = Error;

	fn try_into(self) -> Result<Vec<u8>, Self::Error> {
		use libipld_cbor::DagCborCodec;
		use libipld_core::codec::Codec;

		// libipld_cbor
		let data: Ipld = self.into();
		DagCborCodec.encode(&data)
	}
}

#[cfg(feature = "std")]
impl TryInto<Cid> for ListReference {
	type Error = Error;

	fn try_into(self) -> Result<Cid, Self::Error> {
		use cid::multihash::{Code, MultihashDigest};
		use libipld_cbor::DagCborCodec;

		// serde
		// let DagCborCodec = 0x71;
		// let Sha256 = 0x12;
		// let data = serde_ipld_dagcbor::to_vec(&list_reference).map_err(|e| Error::<T>::GenericEncoding)?;
		// let hash = Code::Sha2_256.digest(&data);
		// Cid::new_v1(DagCborCodec, hash)

		// libipld_cbor
		let data: Vec<u8> = self.try_into()?;
		let hash = Code::Sha2_256.digest(&data);
		Ok(Cid::new_v1(DagCborCodec.into(), hash))
	}
}

#[cfg(feature = "std")]
impl TryInto<(Cid, Vec<u8>)> for ListReference {
	type Error = Error;

	fn try_into(self) -> Result<(Cid, Vec<u8>), Self::Error> {
		use cid::multihash::{Code, MultihashDigest};
		use libipld_cbor::DagCborCodec;
		use libipld_core::codec::Codec;

		// serde
		// let DagCborCodec = 0x71;
		// let Sha256 = 0x12;
		// let data = serde_ipld_dagcbor::to_vec(&list_reference).map_err(|e| Error::<T>::GenericEncoding)?;
		// let hash = Code::Sha2_256.digest(&data);
		// Cid::new_v1(DagCborCodec, hash)

		// libipld_cbor
		let ipld_data: Ipld = self.into();
		let data = DagCborCodec.encode(&ipld_data);
		let hash = Code::Sha2_256.digest(&data);
		Ok((Cid::new_v1(DagCborCodec.into(), hash), data))
	}
}

#[cfg(not(feature = "std"))]
impl TryInto<(Cid, Vec<u8>)> for ListReference {
	type Error = Error;

	fn try_into(self) -> Result<(Cid, Vec<u8>), Self::Error> {
		use sp_cid::sp_multihash::{Code, MultihashDigest};
		use sp_ipld::{dag_cbor::DagCborCodec, Codec};
		let sp_data = into_sp_ipld(self.into());
		let cbor_data = DagCborCodec.encode(&sp_data).map_err(anyhow::Error::msg)?;
		let data = cbor_data.into_inner();
		let hash = Code::Sha2_256.digest(&data);
		let sp_cid = sp_cid::Cid::new_v1(DagCborCodec.into(), hash);
		Ok((into_cid(sp_cid), data))
	}
}

#[cfg(not(feature = "std"))]
impl TryInto<Vec<u8>> for ListReference {
	type Error = Error;

	fn try_into(self) -> Result<Vec<u8>, Self::Error> {
		use sp_ipld::{dag_cbor::DagCborCodec, Codec};
		let sp_data = into_sp_ipld(self.into());
		let cbor_data = DagCborCodec.encode(&sp_data).map_err(anyhow::Error::msg)?;
		Ok(cbor_data.into_inner())
	}
}

#[cfg(not(feature = "std"))]
impl TryInto<Cid> for ListReference {
	type Error = Error;

	fn try_into(self) -> Result<Cid, Self::Error> {
		use sp_cid::sp_multihash::{Code, MultihashDigest};
		use sp_ipld::dag_cbor::DagCborCodec;
		let data: Vec<u8> = self.try_into()?;
		let hash = Code::Sha2_256.digest(&data);
		let sp_cid = sp_cid::Cid::new_v1(DagCborCodec.into(), hash);
		Ok(into_cid(sp_cid))
	}
}

#[cfg(not(feature = "std"))]
fn into_sp_ipld(from: Ipld) -> sp_ipld::Ipld {
	match from {
		Ipld::Bool(v) => sp_ipld::Ipld::Bool(v),
		Ipld::Null => sp_ipld::Ipld::Null,
		Ipld::Integer(v) => sp_ipld::Ipld::Integer(v),
		Ipld::Float(v) => sp_ipld::Ipld::Float(v),
		Ipld::String(v) => sp_ipld::Ipld::String(v),
		Ipld::Bytes(v) => sp_ipld::Ipld::Bytes(v),
		Ipld::List(v) => sp_ipld::Ipld::List(v.into_iter().map(into_sp_ipld).collect()),
		Ipld::Map(v) => sp_ipld::Ipld::StringMap(BTreeMap::from_iter(v.into_iter().map(|(k, v)| (k, into_sp_ipld(v))))),
		Ipld::Link(v) => sp_ipld::Ipld::Link(into_sp_cid(v)),
	}
}

#[cfg(not(feature = "std"))]
fn into_sp_cid(from: Cid) -> sp_cid::Cid {
	sp_cid::Cid::try_from(from.to_bytes()).expect("cid to be compatible")
}

#[cfg(not(feature = "std"))]
fn into_cid(from: sp_cid::Cid) -> Cid {
	Cid::try_from(from.to_bytes()).expect("cid to be compatible")
}
