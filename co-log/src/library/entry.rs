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

	/// The identity.
	#[serde(rename = "s", with = "serde_bytes")]
	pub signature: Vec<u8>,

	/// Entry.
	#[serde(flatten)]
	pub entry: Entry,
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
	pub fn from_entry(identity: &dyn Identity, entry: Entry) -> Result<Self, SerializeError> {
		let block = BlockSerializer::default().serialize(&entry)?;
		let signature = identity.sign(block.data());
		let signed_entry = SignedEntry { identity: identity.identity().to_string(), signature, entry };
		Ok(Self { cid: block.into_inner().0, data: signed_entry })
	}

	pub fn from_unsigned_block(identity: &dyn Identity, block: Block<DefaultParams>) -> Result<Self, SerializeError> {
		let entry = BlockSerializer::default().deserialize(&block)?;
		Self::from_entry(identity, entry)
	}

	pub fn from_signed_entry(entry: SignedEntry) -> Result<Self, SerializeError> {
		let signed_block = BlockSerializer::default().serialize(&entry)?;
		Ok(Self { cid: signed_block.into_inner().0, data: entry })
	}

	pub fn from_signed_block(block: Block<DefaultParams>) -> Result<Self, SerializeError> {
		let entry: SignedEntry = BlockSerializer::default().deserialize(&block)?;
		Ok(Self { cid: block.into_inner().0, data: entry })
	}

	pub fn cid(&self) -> &Cid {
		&self.cid
	}

	pub fn entry(&self) -> &Entry {
		&self.data.entry
	}

	pub fn unsigned_block(&self) -> Result<Block<DefaultParams>, SerializeError> {
		BlockSerializer::default().serialize(self.entry())
	}

	pub fn signed_entry(&self) -> &SignedEntry {
		&self.data
	}

	pub fn block(&self) -> Result<Block<DefaultParams>, SerializeError> {
		BlockSerializer::default().serialize(&self.data)
	}

	pub fn verify(&self, identity: &dyn Identity) -> Result<bool, SerializeError> {
		Ok(identity.verify(&self.data.signature, self.block()?.data(), None))
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
