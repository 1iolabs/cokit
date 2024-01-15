use super::identity::{PrivateIdentity, SignError};
use crate::{Clock, Identity};
use co_storage::{BlockSerializer, SerializeError};
use libipld::{Block, Cid, DefaultParams};
use serde::{Deserialize, Serialize};

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
	Serialize(#[from] SerializeError),

	#[error("Signature failed: {0}")]
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
	pub fn from_entry(identity: &dyn PrivateIdentity, entry: Entry) -> Result<Self, EntryError> {
		let block = BlockSerializer::default().serialize(&entry)?;
		let signature = identity.sign(block.data())?;
		Self::from_signed_entry(SignedEntry {
			identity: identity.identity().to_string(),
			signature,
			entry,
			public_key: identity.public_key(),
		})
	}

	pub fn from_unsigned_block(
		identity: &dyn PrivateIdentity,
		block: Block<DefaultParams>,
	) -> Result<Self, EntryError> {
		let entry = BlockSerializer::default().deserialize(&block)?;
		Self::from_entry(identity, entry)
	}

	pub fn from_signed_entry(entry: SignedEntry) -> Result<Self, EntryError> {
		let signed_block = BlockSerializer::default().serialize(&entry)?;
		Ok(Self { cid: signed_block.into_inner().0, data: entry })
	}

	pub fn from_block(block: Block<DefaultParams>) -> Result<Self, EntryError> {
		let entry: SignedEntry = BlockSerializer::default().deserialize(&block)?;
		Ok(Self { cid: block.into_inner().0, data: entry })
	}

	pub fn cid(&self) -> &Cid {
		&self.cid
	}

	pub fn entry(&self) -> &Entry {
		&self.data.entry
	}

	pub fn unsigned_block(&self) -> Result<Block<DefaultParams>, EntryError> {
		Ok(BlockSerializer::default().serialize(self.entry())?)
	}

	pub fn signed_entry(&self) -> &SignedEntry {
		&self.data
	}

	pub fn block(&self) -> Result<Block<DefaultParams>, EntryError> {
		Ok(BlockSerializer::default().serialize(&self.data)?)
	}

	pub fn verify(&self, identity: &dyn Identity) -> Result<bool, EntryError> {
		Ok(identity.verify(
			&self.data.signature,
			self.unsigned_block()?.data(),
			self.signed_entry().public_key.as_ref().map(Vec::as_slice),
		))
	}
}
impl Into<Entry> for EntryBlock {
	fn into(self) -> Entry {
		self.data.entry
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
		self.cid().partial_cmp(&other.cid())
	}
}
impl Ord for EntryBlock {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.cid().cmp(&other.cid())
	}
}

#[cfg(test)]
mod tests {
	use crate::{Clock, DidKeyIdentity, EntryBlock};
	use co_storage::BlockSerializer;
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
		let entry = EntryBlock::from_entry(
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
