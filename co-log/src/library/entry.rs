use cid::Cid;
use co_identity::{Identity, PrivateIdentity, SignError};
use co_primitives::{
	to_cbor, Block, BlockSerializer, BlockSerializerError, CborError, Entry, SignedEntry, StoreParams,
};

#[derive(Debug, thiserror::Error)]
pub enum EntryError {
	#[error("Block failed.")]
	Serialize(#[from] BlockSerializerError),

	#[error("CBOR failed.")]
	Cbor(#[from] CborError),

	#[error("Signature failed.")]
	Sign(#[from] SignError),
}

/// Deserialized block.
#[derive(Debug, Clone)]
pub struct EntryBlock {
	/// CID of the signed entry.
	cid: Cid,

	/// The entry.
	data: SignedEntry,
}
impl EntryBlock {
	pub fn from_entry<P: StoreParams, I: PrivateIdentity>(identity: &I, entry: Entry) -> Result<Self, EntryError> {
		let data = to_cbor(&entry)?;
		let signature = identity.sign(&data)?;
		Self::from_signed_entry::<P>(SignedEntry {
			identity: identity.identity().to_string(),
			signature,
			entry,
			public_key: identity.public_key(),
		})
	}

	pub fn from_unsigned_block<P: StoreParams, I: PrivateIdentity>(
		identity: &I,
		block: Block<P>,
	) -> Result<Self, EntryError> {
		let entry = BlockSerializer::<P>::new().deserialize(&block)?;
		Self::from_entry::<P, I>(identity, entry)
	}

	pub fn from_signed_entry<P: StoreParams>(entry: SignedEntry) -> Result<Self, EntryError> {
		let signed_block = BlockSerializer::<P>::new().serialize(&entry)?;
		Ok(Self { cid: signed_block.into_inner().0, data: entry })
	}

	pub fn from_block<P: StoreParams>(block: Block<P>) -> Result<Self, EntryError> {
		let entry: SignedEntry = BlockSerializer::<P>::new().deserialize(&block)?;
		Ok(Self { cid: block.into_inner().0, data: entry })
	}

	pub fn cid(&self) -> &Cid {
		&self.cid
	}

	pub fn entry(&self) -> &Entry {
		&self.data.entry
	}

	pub fn unsigned_data(&self) -> Result<Vec<u8>, EntryError> {
		Ok(to_cbor(self.entry())?)
	}

	pub fn unsigned_block<P: StoreParams>(&self) -> Result<Block<P>, EntryError> {
		Ok(BlockSerializer::<P>::new().serialize(self.entry())?)
	}

	pub fn signed_entry(&self) -> &SignedEntry {
		&self.data
	}

	pub fn block<P: StoreParams>(&self) -> Result<Block<P>, EntryError> {
		Ok(BlockSerializer::new().serialize(&self.data)?)
	}

	pub fn verify(&self, identity: &dyn Identity) -> Result<bool, EntryError> {
		Ok(identity.verify(&self.data.signature, &self.unsigned_data()?, self.signed_entry().public_key.as_deref()))
	}
}
impl PartialEq for EntryBlock {
	fn eq(&self, other: &Self) -> bool {
		self.cid() == other.cid()
	}
}
impl Eq for EntryBlock {}
impl PartialOrd for EntryBlock {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		self.cid().partial_cmp(other.cid())
	}
}
impl Ord for EntryBlock {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.cid().cmp(other.cid())
	}
}

#[cfg(test)]
mod tests {
	use crate::EntryBlock;
	use co_identity::DidKeyIdentity;
	use co_primitives::{BlockSerializer, Clock, DefaultParams, Entry};
	use serde::{Deserialize, Serialize};

	#[test]
	fn smoke() {
		#[derive(Debug, Serialize, Deserialize)]
		struct Event {
			#[serde(rename = "type")]
			t: String,
		}

		//data
		let data = Event { t: "hello".to_string() };
		let block = BlockSerializer::default().serialize(&data).unwrap();

		// entry
		let identity = Box::new(DidKeyIdentity::generate(None));
		let entry = EntryBlock::from_entry::<DefaultParams, _>(
			identity.as_ref(),
			Entry {
				id: vec![0],
				payload: *block.cid(),
				next: Default::default(),
				refs: Default::default(),
				clock: Clock::new(vec![1], 0),
			},
		)
		.unwrap();

		// serialize
		let signed_block = entry.block::<DefaultParams>().unwrap();

		// deserialize
		let entry_desertialized = EntryBlock::from_block(signed_block).unwrap();

		// check
		assert_eq!(entry.entry(), entry_desertialized.entry());

		// verify
		assert!(entry.verify(identity.as_ref()).unwrap());
	}
}
