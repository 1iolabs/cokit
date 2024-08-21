use crate::types::{
	block::{BlockStat, BlockStorage},
	storage::{Storage, StorageError},
};
use anyhow::anyhow;
use async_trait::async_trait;
use libipld::{Block, Cid, DefaultParams};
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::RwLock;

pub struct MemoryStorage {
	records: BTreeMap<Cid, Record>,
}

impl Default for MemoryStorage {
	fn default() -> Self {
		Self::new()
	}
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
		self.records.keys()
	}
}

impl Storage for MemoryStorage {
	type StoreParams = DefaultParams;

	fn set(&mut self, block: Block<DefaultParams>) -> Result<Cid, StorageError> {
		// let cid = Cid::new_v1(options.codec, Code::Blake3_256.digest(&data[..]));
		tracing::debug!(cid = ?block.cid(), "memory-store-set");
		let result = *block.cid();
		self.records.insert(*block.cid(), Record { pin: false, block });
		Ok(result)
	}

	fn get(&self, cid: &Cid) -> Result<Block<DefaultParams>, StorageError> {
		let result = self
			.records
			.get(cid)
			.map(|r| r.block.clone())
			.ok_or(StorageError::NotFound(*cid, anyhow!("no record")));
		tracing::debug!(?cid, return = ?result.as_ref().map(|_| ()), "memory-store-get");
		result
	}

	fn remove(&mut self, cid: &Cid) -> Result<(), StorageError> {
		tracing::debug!(?cid, "memory-store-remove");
		self.records.remove(cid);
		Ok(())
	}
}

#[derive(Debug, Clone)]
pub struct MemoryBlockStorage {
	records: Arc<RwLock<BTreeMap<Cid, Record>>>,
}

impl Default for MemoryBlockStorage {
	fn default() -> Self {
		Self::new()
	}
}

impl MemoryBlockStorage {
	pub fn new() -> Self {
		Self { records: Default::default() }
	}
}

#[async_trait]
impl BlockStorage for MemoryBlockStorage {
	type StoreParams = DefaultParams;

	async fn get(&self, cid: &Cid) -> Result<Block<Self::StoreParams>, StorageError> {
		let result = self
			.records
			.read()
			.await
			.get(cid)
			.map(|r| r.block.clone())
			.ok_or(StorageError::NotFound(*cid, anyhow!("no record")));
		tracing::trace!(?cid, return = ?result.as_ref().map(|_| ()), "memory-store-get");
		result
	}

	async fn set(&self, block: Block<Self::StoreParams>) -> Result<Cid, StorageError> {
		tracing::trace!(cid = ?block.cid(), "memory-store-set");
		let result = *block.cid();
		self.records.write().await.insert(*block.cid(), Record { pin: false, block });
		Ok(result)
	}

	async fn remove(&self, cid: &Cid) -> Result<(), StorageError> {
		tracing::trace!(?cid, "memory-store-remove");
		self.records.write().await.remove(cid);
		Ok(())
	}

	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError> {
		self.records
			.read()
			.await
			.get(cid)
			.map(|r| BlockStat { size: r.block.data().len() as u64 })
			.ok_or(StorageError::NotFound(*cid, anyhow!("no record")))
	}
}

#[derive(Debug, Clone)]
struct Record {
	block: Block<DefaultParams>,
	pin: bool,
}
