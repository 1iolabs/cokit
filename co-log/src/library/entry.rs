use crate::{Clock, Identity};
use co_storage::{BlockSerializer, SerializeError};
use libipld::{Block, Cid, DefaultParams};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Entry {
	/// The stream id.
	/// Todo: Do we need this?
	#[serde(rename = "i")]
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
	#[serde(rename = "s")]
	pub signature: Vec<u8>,

	/// Entry.
	#[serde(flatten)]
	pub entry: Entry,
}

#[derive(Debug, Clone)]
enum EntryBlockPayload {
	Entry(Entry),
	SignedEntry(Cid, SignedEntry),
}
impl Into<Entry> for EntryBlockPayload {
	fn into(self) -> Entry {
		match self {
			EntryBlockPayload::Entry(e) => e,
			EntryBlockPayload::SignedEntry(_, e) => e.entry,
		}
	}
}

/// Deserialized block.
#[derive(Debug, Clone)]
pub struct EntryBlock {
	cid: Cid,
	data: EntryBlockPayload,
}
impl EntryBlock {
	pub fn from_entry(entry: Entry) -> Result<Self, SerializeError> {
		let block = BlockSerializer::default().serialize(&entry)?;
		Ok(Self { cid: block.into_inner().0, data: EntryBlockPayload::Entry(entry) })
	}

	pub fn from_signed_entry(entry: SignedEntry) -> Result<Self, SerializeError> {
		let signed_block = BlockSerializer::default().serialize(&entry)?;
		let block = BlockSerializer::default().serialize(&entry.entry)?;
		Ok(Self { cid: block.into_inner().0, data: EntryBlockPayload::SignedEntry(signed_block.into_inner().0, entry) })
	}

	pub fn from_block(identity: &dyn Identity, block: Block<DefaultParams>) -> Result<Self, SerializeError> {
		let entry = BlockSerializer::default().deserialize(&block)?;
		Ok(Self { cid: block.into_inner().0, data: EntryBlockPayload::Entry(entry) })
	}

	pub fn from_signed_block(block: Block<DefaultParams>) -> Result<Self, SerializeError> {
		let entry: SignedEntry = BlockSerializer::default().deserialize(&block)?;
		let unsigned_block = BlockSerializer::default().serialize(&entry.entry)?;
		Ok(Self {
			cid: unsigned_block.into_inner().0,
			data: EntryBlockPayload::SignedEntry(block.into_inner().0, entry),
		})
	}

	pub fn cid(&self) -> &Cid {
		&self.cid
	}

	pub fn entry(&self) -> &Entry {
		match &self.data {
			EntryBlockPayload::Entry(e) => e,
			EntryBlockPayload::SignedEntry(_, e) => &e.entry,
		}
	}

	pub fn block(&self) -> Result<Block<DefaultParams>, SerializeError> {
		BlockSerializer::default().serialize(self.entry())
	}

	pub fn is_signed(&self) -> bool {
		self.signed_entry().is_some()
	}

	pub fn signed_cid(&self) -> Option<&Cid> {
		match &self.data {
			EntryBlockPayload::Entry(_) => None,
			EntryBlockPayload::SignedEntry(i, _) => Some(i),
		}
	}

	pub fn signed_entry(&self) -> Option<&SignedEntry> {
		match &self.data {
			EntryBlockPayload::Entry(_) => None,
			EntryBlockPayload::SignedEntry(_, e) => Some(&e),
		}
	}

	pub fn signed_block(&self) -> Option<Result<Block<DefaultParams>, SerializeError>> {
		if let Some(e) = self.signed_entry() {
			Some(BlockSerializer::default().serialize(e))
		} else {
			None
		}
	}

	pub fn sign(&mut self, identity: &dyn Identity) -> Result<(), SerializeError> {
		let block = self.block()?;
		self.data = EntryBlockPayload::SignedEntry(
			block.cid().clone(),
			SignedEntry {
				identity: identity.identity(),
				signature: identity.sign(block.data()),
				entry: self.entry().clone(),
			},
		);
		Ok(())
	}

	/// Returns None if not signed.
	pub fn verify(&self, identity: &dyn Identity) -> Option<Result<bool, SerializeError>> {
		match &self.data {
			EntryBlockPayload::Entry(_) => None,
			EntryBlockPayload::SignedEntry(_, e) => match self.block() {
				Ok(b) => Some(Ok(identity.verify(&e.signature, b.data(), None))),
				Err(e) => Some(Err(e)),
			},
		}
	}
}
impl Into<Entry> for EntryBlock {
	fn into(self) -> Entry {
		match self.data {
			EntryBlockPayload::Entry(e) => e,
			EntryBlockPayload::SignedEntry(_, e) => e.entry,
		}
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
