// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{ExtendedBlock, ExtendedBlockStorage};
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{Block, BlockStat, BlockStorage, KnownMultiCodec, StorageError};
use std::{collections::BTreeMap, ops::Range};

/// Block storage implementation for static data.
#[derive(Debug, Clone)]
pub struct StaticBlockStorage<'a> {
	data: &'a [u8],
	blocks: BTreeMap<Cid, StaticBlock>,
}
impl<'a> StaticBlockStorage<'a> {
	/// Static block storage builder.
	pub fn builder(data: &'a [u8]) -> StaticBlockStorageBuilder<'a> {
		StaticBlockStorageBuilder::new(data)
	}

	/// Create static block storage from plain block.
	pub fn new_unchecked(cid: Cid, data: &'a [u8]) -> Self {
		Self { blocks: [(cid, StaticBlock::Range(0..data.len()))].into_iter().collect(), data }
	}

	/// Create static block storage from raw block.
	pub fn new_raw(data: &'a [u8]) -> (Cid, Self) {
		let cid = Block::cid_data(KnownMultiCodec::Raw, data);
		let storage = Self { blocks: [(cid, StaticBlock::Range(0..data.len()))].into_iter().collect(), data };
		(cid, storage)
	}
}
#[async_trait]
impl<'a> BlockStorage for StaticBlockStorage<'a> {
	async fn get(&self, cid: &Cid) -> Result<Block, StorageError> {
		let block = self
			.blocks
			.get(cid)
			.ok_or_else(|| StorageError::NotFound(*cid, anyhow::anyhow!("Block not found")))?;
		Ok(Block::new_unchecked(*cid, block.to_vec(self.data)))
	}

	async fn set(&self, _block: Block) -> Result<Cid, StorageError> {
		Err(StorageError::InvalidArgument(anyhow::anyhow!("Readonly storage")))
	}

	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError> {
		let block = self
			.blocks
			.get(cid)
			.ok_or_else(|| StorageError::NotFound(*cid, anyhow::anyhow!("Block not found")))?;
		Ok(BlockStat { size: block.len() as u64 })
	}

	async fn remove(&self, _cid: &Cid) -> Result<(), StorageError> {
		Err(StorageError::InvalidArgument(anyhow::anyhow!("Readonly storage")))
	}

	fn max_block_size(&self) -> usize {
		0
	}
}
#[async_trait]
impl<'a> ExtendedBlockStorage for StaticBlockStorage<'a> {
	async fn set_extended(&self, _block: ExtendedBlock) -> Result<Cid, StorageError> {
		Err(StorageError::InvalidArgument(anyhow::anyhow!("Readonly storage")))
	}

	async fn exists(&self, cid: &Cid) -> Result<bool, StorageError> {
		Ok(self.blocks.contains_key(cid))
	}

	async fn clear(&self) -> Result<(), StorageError> {
		Err(StorageError::InvalidArgument(anyhow::anyhow!("Readonly storage")))
	}
}

#[derive(Debug, Clone)]
enum StaticBlock {
	Range(Range<usize>),
	Data(Vec<u8>),
}
impl StaticBlock {
	pub fn to_vec(&self, data: &[u8]) -> Vec<u8> {
		match self {
			Self::Range(range) => data[range.start..range.end].to_vec(),
			Self::Data(data) => data.clone(),
		}
	}

	pub fn len(&self) -> usize {
		match self {
			StaticBlock::Range(range) => range.len(),
			StaticBlock::Data(data) => data.len(),
		}
	}
}

pub struct StaticBlockStorageBuilder<'a> {
	data: &'a [u8],
	blocks: BTreeMap<Cid, StaticBlock>,
}
impl<'a> StaticBlockStorageBuilder<'a> {
	pub fn new(data: &'a [u8]) -> Self {
		Self { data, blocks: Default::default() }
	}

	pub fn with_range(mut self, cid: Cid, range: Range<usize>) -> Self {
		self.blocks.insert(cid, StaticBlock::Range(range));
		self
	}

	pub fn with_block(mut self, cid: Cid, data: Vec<u8>) -> Self {
		self.blocks.insert(cid, StaticBlock::Data(data));
		self
	}

	pub fn build(self) -> StaticBlockStorage<'a> {
		StaticBlockStorage { blocks: self.blocks, data: self.data }
	}
}
