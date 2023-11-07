use crate::types::{
	codec::MultiCodec,
	storage::{Storage, StorageError},
};
use libipld::{Block, Cid, DefaultParams};
use std::{collections::BTreeMap, fmt::Debug};

pub struct MemoryStorage {
	records: BTreeMap<Cid, Record>,
}

impl MemoryStorage {
	pub fn new() -> Self {
		Self { records: BTreeMap::new() }
	}

	pub fn pin(&mut self, cid: &Cid) -> bool {
		match self.records.get_mut(cid) {
			Some(r) => {
				r.pin = true;
				true
			},
			None => false,
		}
	}

	pub fn unpin(&mut self, cid: &Cid) -> bool {
		match self.records.get_mut(cid) {
			Some(r) => {
				r.pin = false;
				true
			},
			None => false,
		}
	}
}

impl Storage for MemoryStorage {
	fn set(&mut self, block: Block<DefaultParams>) -> Result<(), StorageError> {
		// let cid = Cid::new_v1(options.codec, Code::Blake3_256.digest(&data[..]));
		self.records.insert(block.cid().clone(), Record { pin: false, block });
		Ok(())
	}

	fn get(&self, cid: &Cid) -> Result<Block<DefaultParams>, StorageError> {
		self.records.get(cid).map(|r| r.block.clone()).ok_or(StorageError::NotFound)
	}
}

struct Record {
	block: Block<DefaultParams>,
	pin: bool,
}

#[derive(Debug, Clone)]
pub struct AddOptions {
	pin: bool,
	codec: u64,
}
impl AddOptions {
	pub fn new(codec: u64) -> Self {
		Self { pin: Default::default(), codec }
	}
}
impl Default for AddOptions {
	fn default() -> Self {
		Self { pin: Default::default(), codec: MultiCodec::Raw as u64 }
	}
}
