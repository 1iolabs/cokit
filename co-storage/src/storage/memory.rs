use crate::{types::storage::Storage, BlockStorageContentMapping};
use anyhow::anyhow;
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{
	Block, BlockStat, BlockStorage, BlockStorageSettings, CloneWithBlockStorageSettings, DefaultParams, StorageError,
	StoreParams,
};
use std::{
	collections::BTreeMap,
	sync::{Arc, RwLock},
};

#[derive(Debug)]
pub struct MemoryStorage {
	records: BTreeMap<Cid, Record<DefaultParams>>,
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
pub struct MemoryBlockStorage<P = DefaultParams>
where
	P: StoreParams,
{
	records: Arc<RwLock<BTreeMap<Cid, Record<P>>>>,
}
impl<P> MemoryBlockStorage<P>
where
	P: StoreParams,
{
	pub fn new() -> Self {
		Self { records: Default::default() }
	}

	pub async fn is_empty(&self) -> bool {
		self.records.read().unwrap().is_empty()
	}

	pub async fn entries(&self) -> impl Iterator<Item = Block<P>> + use<P> {
		let records = { self.records.read().unwrap().clone() };
		records.into_iter().map(|(_, record)| record.block)
	}
}
impl Default for MemoryBlockStorage<DefaultParams> {
	fn default() -> Self {
		Self::new()
	}
}
#[async_trait]
impl<P> BlockStorage for MemoryBlockStorage<P>
where
	P: StoreParams,
{
	type StoreParams = P;

	async fn get(&self, cid: &Cid) -> Result<Block<Self::StoreParams>, StorageError> {
		let result = self
			.records
			.read()
			.unwrap()
			.get(cid)
			.map(|r| r.block.clone())
			.ok_or(StorageError::NotFound(*cid, anyhow!("no record")));
		tracing::trace!(?cid, return = ?result.as_ref().map(|_| ()), "memory-store-get");
		result
	}

	async fn set(&self, block: Block<Self::StoreParams>) -> Result<Cid, StorageError> {
		tracing::trace!(cid = ?block.cid(), "memory-store-set");
		let result = *block.cid();
		self.records.write().unwrap().insert(*block.cid(), Record { pin: false, block });
		Ok(result)
	}

	async fn remove(&self, cid: &Cid) -> Result<(), StorageError> {
		tracing::trace!(?cid, "memory-store-remove");
		self.records.write().unwrap().remove(cid);
		Ok(())
	}

	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError> {
		self.records
			.read()
			.unwrap()
			.get(cid)
			.map(|r| BlockStat { size: r.block.data().len() as u64 })
			.ok_or(StorageError::NotFound(*cid, anyhow!("no record")))
	}
}
impl<P> CloneWithBlockStorageSettings for MemoryBlockStorage<P>
where
	P: StoreParams,
{
	fn clone_with_settings(&self, _settings: BlockStorageSettings) -> Self {
		self.clone()
	}
}
#[async_trait]
impl BlockStorageContentMapping for MemoryBlockStorage {}

#[derive(Debug, Clone)]
struct Record<P> {
	block: Block<P>,
	pin: bool,
}
