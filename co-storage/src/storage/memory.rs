use crate::types::{
	block::{BlockStat, BlockStorage},
	storage::{Storage, StorageError},
};
use async_trait::async_trait;
use libipld::{Block, Cid, DefaultParams};
use std::collections::BTreeMap;

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

	/// Iterator over all stored CIDs.
	pub fn iter(&self) -> impl Iterator<Item = &Cid> {
		self.records.iter().map(|(cid, _)| cid)
	}
}

impl Storage for MemoryStorage {
	fn set(&mut self, block: Block<DefaultParams>) -> Result<(), StorageError> {
		// let cid = Cid::new_v1(options.codec, Code::Blake3_256.digest(&data[..]));
		tracing::debug!(cid = ?block.cid(), "memory-store-set");
		self.records.insert(block.cid().clone(), Record { pin: false, block });
		Ok(())
	}

	fn get(&self, cid: &Cid) -> Result<Block<DefaultParams>, StorageError> {
		tracing::debug!(?cid, "memory-store-get");
		self.records
			.get(cid)
			.map(|r| r.block.clone())
			.ok_or(StorageError::NotFound(cid.clone()))
	}

	fn remove(&mut self, cid: &Cid) -> Result<(), StorageError> {
		tracing::debug!(?cid, "memory-store-remove");
		self.records.remove(cid);
		Ok(())
	}
}

#[async_trait(?Send)]
impl BlockStorage for MemoryStorage {
	type StoreParams = DefaultParams;

	async fn get(&self, cid: &Cid) -> Result<Block<DefaultParams>, StorageError> {
		Storage::get(self, cid)
	}

	async fn set(&mut self, block: Block<DefaultParams>) -> Result<(), StorageError> {
		Storage::set(self, block)
	}

	async fn remove(&mut self, cid: &Cid) -> Result<(), StorageError> {
		Storage::remove(self, cid)
	}

	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError> {
		self.records
			.get(cid)
			.map(|r| BlockStat { size: r.block.data().len() as u64 })
			.ok_or(StorageError::NotFound(cid.clone()))
	}
}

struct Record {
	block: Block<DefaultParams>,
	pin: bool,
}
