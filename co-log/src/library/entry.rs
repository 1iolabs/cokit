use crate::{Clock, Identity, PrivateIdentity, SignError};
use co_primitives::{BlockSerializer, BlockSerializerError};
use libipld::{store::StoreParams, Block, Cid};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Entry {
	/// The stream id.
	/// Todo: Do we need this?
	#[serde(rename = "i", with = "serde_bytes")]
	pub id: Vec<u8>,
	#[serde(rename = "p")]
	pub payload: Cid,
	#[serde(rename = "n")]
	pub next: Vec<Cid>,
	#[serde(rename = "r", default, skip_serializing_if = "Vec::is_empty")]
	pub refs: Vec<Cid>,
	#[serde(rename = "c")]
	pub clock: Clock,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SignedEntry {
	/// The identity.
	#[serde(rename = "u")]
	pub identity: String,

	/// Identity public key.
	#[serde(rename = "k", default, with = "serde_bytes", skip_serializing_if = "Option::is_none")]
	pub public_key: Option<Vec<u8>>,

	/// The identity.
	#[serde(rename = "s", with = "serde_bytes")]
	pub signature: Vec<u8>,

	/// Entry.
	#[serde(rename = "e")]
	// note: this causes serde to write unbounded maps which are indefinite length maps which are not supported in
	// DAG-CBOR. #[serde(flatten)]
	pub entry: Entry,
}

#[derive(Debug, thiserror::Error)]
pub enum EntryError {
	#[error("Serialize failed: {0}")]
	Serialize(#[from] BlockSerializerError),

	#[error("Signature failed: {0}")]
	Sign(#[from] SignError),
}

/// Deserialized block.
#[derive(Debug, Clone)]
pub struct EntryBlock<P> {
	_p: PhantomData<P>,

	/// CID of the signed entry.
	cid: Cid,

	/// The entry.
	data: SignedEntry,
}
impl<P: StoreParams> EntryBlock<P> {
	pub fn from_entry(identity: &dyn PrivateIdentity, entry: Entry) -> Result<Self, EntryError> {
		let block = BlockSerializer::<P>::new().serialize(&entry)?;
		let signature = identity.sign(block.data())?;
		Self::from_signed_entry(SignedEntry {
			identity: identity.identity().to_string(),
			signature,
			entry,
			public_key: identity.public_key(),
		})
	}

	pub fn from_unsigned_block(identity: &dyn PrivateIdentity, block: Block<P>) -> Result<Self, EntryError> {
		let entry = BlockSerializer::<P>::new().deserialize(&block)?;
		Self::from_entry(identity, entry)
	}

	pub fn from_signed_entry(entry: SignedEntry) -> Result<Self, EntryError> {
		let signed_block = BlockSerializer::<P>::new().serialize(&entry)?;
		Ok(Self { _p: Default::default(), cid: signed_block.into_inner().0, data: entry })
	}

	pub fn from_block(block: Block<P>) -> Result<Self, EntryError> {
		let entry: SignedEntry = BlockSerializer::<P>::new().deserialize(&block)?;
		Ok(Self { _p: Default::default(), cid: block.into_inner().0, data: entry })
	}

	pub fn cid(&self) -> &Cid {
		&self.cid
	}

	pub fn entry(&self) -> &Entry {
		&self.data.entry
	}

	pub fn unsigned_block(&self) -> Result<Block<P>, EntryError> {
		Ok(BlockSerializer::<P>::new().serialize(self.entry())?)
	}

	pub fn signed_entry(&self) -> &SignedEntry {
		&self.data
	}

	pub fn block(&self) -> Result<Block<P>, EntryError> {
		Ok(BlockSerializer::new().serialize(&self.data)?)
	}

	pub fn verify(&self, identity: &dyn Identity) -> Result<bool, EntryError> {
		Ok(identity.verify(
			&self.data.signature,
			self.unsigned_block()?.data(),
			self.signed_entry().public_key.as_ref().map(Vec::as_slice),
		))
	}
}
impl<P: StoreParams> Into<Entry> for EntryBlock<P> {
	fn into(self) -> Entry {
		self.data.entry
	}
}
impl<P: StoreParams> PartialEq for EntryBlock<P> {
	fn eq(&self, other: &Self) -> bool {
		self.cid() == other.cid()
	}
}
impl<P: StoreParams> Eq for EntryBlock<P> {}
impl<P: StoreParams> PartialOrd for EntryBlock<P> {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		self.cid().partial_cmp(&other.cid())
	}
}
impl<P: StoreParams> Ord for EntryBlock<P> {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.cid().cmp(&other.cid())
	}
}

#[cfg(test)]
mod tests {
	use crate::{Clock, DidKeyIdentity, EntryBlock};
	use co_primitives::BlockSerializer;
	use libipld::DefaultParams;
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
		let entry = EntryBlock::<DefaultParams>::from_entry(
			identity.as_ref(),
			crate::Entry {
				id: vec![0],
				payload: block.cid().clone(),
				next: vec![],
				refs: vec![],
				clock: Clock::new(vec![1], 0),
			},
		)
		.unwrap();

		// serialize
		let signed_block = entry.block().unwrap();

		// deserialize
		let entry_desertialized = EntryBlock::from_block(signed_block).unwrap();

		// check
		assert_eq!(entry.entry(), entry_desertialized.entry());

		// verify
		assert!(entry.verify(identity.as_ref()).unwrap());
	}
}
