use self::{block::Algorithm, secret::Secret};
use crate::types::{codec::MultiCodec, storage::Storage};
use libipld::{
	multihash::{Code, MultihashDigest},
	Block, Cid, DefaultParams,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, default, fmt::Debug};

pub mod block;
pub mod secret;

struct EncryptedStorage {
	key: Secret,
	algorithm: Algorithm,
	next: Box<dyn Storage>,
}

impl Storage for EncryptedStorage {
	fn get(&self, cid: &Cid) -> Block<DefaultParams> {
		todo!()
	}

	fn set(&mut self, block: Block<DefaultParams>) {
		todo!()
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

pub struct MemoryStorage {
	records: BTreeMap<Cid, Record>,
}

impl MemoryStorage {
	pub fn new() -> Self {
		Self { records: BTreeMap::new() }
	}

	pub fn set(&mut self, block: Block<DefaultParams>) {
		// let cid = Cid::new_v1(options.codec, Code::Blake3_256.digest(&data[..]));
		self.records.insert(block.cid().clone(), Record { pin: false, block });
	}

	pub fn get(&self, cid: &Cid) -> Option<Block<DefaultParams>> {
		self.records.get(cid).map(|r| r.block.clone())
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
