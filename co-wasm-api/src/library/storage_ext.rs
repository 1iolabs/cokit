use super::resolve_link::ResolveError;
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
// impl<T> StorageExt for &T where T: Storage + ?Sized {}
