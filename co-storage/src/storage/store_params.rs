// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{BlockStat, BlockStorage, BlockStorageContentMapping, ExtendedBlock, ExtendedBlockStorage, StorageError};
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{Block, BlockStorageCloneSettings, CloneWithBlockStorageSettings, MappedCid};
use std::collections::BTreeSet;

/// This storage implementation converts block storeparams.
/// If an conversation is not possible `StorageError::InvalidArgument` is retuned.
#[derive(Debug, Clone)]
pub struct StoreParamsBlockStorage<S>
where
	S: Clone,
{
	next: S,
	checked: bool,
	max_block_size: usize,
}
impl<S> StoreParamsBlockStorage<S>
where
	S: Clone,
{
	pub fn new(next: S, checked: bool, max_block_size: usize) -> Self {
		Self { next, checked, max_block_size }
	}
}
#[async_trait]
impl<S> BlockStorage for StoreParamsBlockStorage<S>
where
	S: BlockStorage + Send + Sync + Clone,
{
	async fn get(&self, cid: &Cid) -> Result<Block, StorageError> {
		let (cid, data) = self.next.get(cid).await?.with_block_max_size(self.max_block_size)?.into_inner();
		match self.checked {
			true => Ok(Block::new(cid, data)?),
			false => Ok(Block::new_unchecked(cid, data)),
		}
	}

	async fn set(&self, block: Block) -> Result<Cid, StorageError> {
		self.next
			.set(if self.checked { block.with_block_max_size(self.max_block_size)? } else { block })
			.await
	}

	async fn remove(&self, cid: &Cid) -> Result<(), StorageError> {
		self.next.remove(cid).await
	}

	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError> {
		self.next.stat(cid).await
	}

	fn max_block_size(&self) -> usize {
		self.max_block_size
	}
}
#[async_trait]
impl<S> ExtendedBlockStorage for StoreParamsBlockStorage<S>
where
	S: ExtendedBlockStorage + Send + Sync + Clone,
{
	async fn set_extended(&self, block: ExtendedBlock) -> Result<Cid, StorageError> {
		let next_block = ExtendedBlock {
			block: if self.checked { block.block.with_block_max_size(self.max_block_size)? } else { block.block },
			options: block.options,
		};
		self.next.set_extended(next_block).await
	}

	async fn exists(&self, cid: &Cid) -> Result<bool, StorageError> {
		self.next.exists(cid).await
	}

	async fn clear(&self) -> Result<(), StorageError> {
		self.next.clear().await
	}
}
impl<S> CloneWithBlockStorageSettings for StoreParamsBlockStorage<S>
where
	S: BlockStorage + CloneWithBlockStorageSettings,
{
	fn clone_with_settings(&self, settings: BlockStorageCloneSettings) -> Self {
		Self {
			next: self.next.clone_with_settings(settings),
			checked: self.checked,
			max_block_size: self.max_block_size,
		}
	}
}
#[async_trait]
impl<S> BlockStorageContentMapping for StoreParamsBlockStorage<S>
where
	S: BlockStorage + CloneWithBlockStorageSettings + BlockStorageContentMapping + 'static,
{
	async fn is_content_mapped(&self) -> bool {
		self.next.is_content_mapped().await
	}

	async fn to_plain(&self, mapped: &Cid) -> Option<Cid> {
		self.next.to_plain(mapped).await
	}

	async fn to_mapped(&self, plain: &Cid) -> Option<Cid> {
		self.next.to_mapped(plain).await
	}

	async fn insert_mappings(&self, mappings: BTreeSet<MappedCid>) {
		self.next.insert_mappings(mappings).await
	}
}
