// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{types::storage::Storage, BlockStorageContentMapping, ExtendedBlock, ExtendedBlockStorage};
use anyhow::anyhow;
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{
	Block, BlockStat, BlockStorage, BlockStorageCloneSettings, CloneWithBlockStorageSettings, DefaultParams,
	StorageError, StoreParams,
};
use std::{
	collections::BTreeMap,
	sync::{Arc, RwLock},
};

#[derive(Debug)]
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

	fn set(&mut self, block: Block) -> Result<Cid, StorageError> {
		// let cid = Cid::new_v1(options.codec, Code::Blake3_256.digest(&data[..]));
		tracing::debug!(cid = ?block.cid(), "memory-store-set");
		let result = *block.cid();
		self.records
			.insert(*block.cid(), Record { pin: false, block: block.with_store_params::<Self::StoreParams>()? });
		Ok(result)
	}

	fn get(&self, cid: &Cid) -> Result<Block, StorageError> {
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
	max_block_size: usize,
}
impl MemoryBlockStorage {
	pub fn new() -> Self {
		Self { records: Default::default(), max_block_size: DefaultParams::MAX_BLOCK_SIZE }
	}

	pub fn with_max_block_size(mut self, max_block_size: usize) -> Self {
		self.max_block_size = max_block_size;
		self
	}

	pub async fn is_empty(&self) -> bool {
		self.records.read().unwrap().is_empty()
	}

	pub async fn entries(&self) -> impl Iterator<Item = Block> {
		let records = { self.records.read().unwrap().clone() };
		records.into_values().map(|record| record.block)
	}
}
impl Default for MemoryBlockStorage {
	fn default() -> Self {
		Self::new()
	}
}
#[async_trait]
impl BlockStorage for MemoryBlockStorage {
	async fn get(&self, cid: &Cid) -> Result<Block, StorageError> {
		let result = self
			.records
			.read()
			.unwrap()
			.get(cid)
			.map(|r| r.block.clone())
			.ok_or(StorageError::NotFound(*cid, anyhow!("no record")));
		#[cfg(feature = "logging-verbose")]
		tracing::trace!(?cid, result = ?result.as_ref().map(|_| ()), "memory-store-get");
		result
	}

	async fn set(&self, block: Block) -> Result<Cid, StorageError> {
		// log
		#[cfg(feature = "logging-verbose")]
		{
			if co_primitives::MultiCodec::is_cbor(block.cid()) {
				tracing::trace!(cid = ?block.cid(), ipld = ?co_primitives::from_cbor::<ipld_core::ipld::Ipld>(block.data()), "memory-store-set");
			} else {
				tracing::trace!(cid = ?block.cid(), "memory-store-set");
			}
		}

		// apply
		let result = *block.cid();
		self.records.write().unwrap().insert(*block.cid(), Record { pin: false, block });

		// result
		Ok(result)
	}

	async fn remove(&self, cid: &Cid) -> Result<(), StorageError> {
		// log
		#[cfg(feature = "logging-verbose")]
		tracing::trace!(?cid, "memory-store-remove");

		// apply
		self.records.write().unwrap().remove(cid);
		Ok(())
	}

	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError> {
		let result = self
			.records
			.read()
			.unwrap()
			.get(cid)
			.map(|r| BlockStat { size: r.block.data().len() as u64 })
			.ok_or(StorageError::NotFound(*cid, anyhow!("no record")));

		// log
		#[cfg(feature = "logging-verbose")]
		tracing::trace!(?cid, ?result, "memory-store-stat");

		// result
		result
	}

	fn max_block_size(&self) -> usize {
		self.max_block_size
	}
}
#[async_trait]
impl ExtendedBlockStorage for MemoryBlockStorage {
	async fn set_extended(&self, block: ExtendedBlock) -> Result<Cid, StorageError> {
		self.set(block.block).await
	}

	async fn exists(&self, cid: &Cid) -> Result<bool, StorageError> {
		let result = Ok(self.records.read().unwrap().contains_key(cid));

		// log
		#[cfg(feature = "logging-verbose")]
		tracing::trace!(?cid, ?result, "memory-store-exists");

		// result
		result
	}

	async fn clear(&self) -> Result<(), StorageError> {
		self.records.write().unwrap().clear();
		Ok(())
	}
}
impl CloneWithBlockStorageSettings for MemoryBlockStorage {
	fn clone_with_settings(&self, _settings: BlockStorageCloneSettings) -> Self {
		self.clone()
	}
}
#[async_trait]
impl BlockStorageContentMapping for MemoryBlockStorage {}

#[derive(Debug, Clone)]
struct Record {
	block: Block,
	pin: bool,
}
