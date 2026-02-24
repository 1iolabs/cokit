// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use async_trait::async_trait;
use cid::Cid;
use co_primitives::{Block, BlockStorageCloneSettings, CloneWithBlockStorageSettings, MappedCid};
use co_storage::{
	BlockStat, BlockStorage, BlockStorageContentMapping, ExtendedBlock, ExtendedBlockStorage, StorageError,
};
use std::{collections::BTreeSet, fmt::Debug, sync::Arc};

/// Public storage API.
#[derive(Clone)]
pub struct CoStorage {
	inner: Arc<dyn CoStorageBlockStorage>,
}
impl CoStorage {
	pub fn new<S>(storage: S) -> Self
	where
		S: BlockStorage + ExtendedBlockStorage + BlockStorageContentMapping + CloneWithBlockStorageSettings + 'static,
	{
		Self { inner: Arc::new(storage) }
	}
}
impl Debug for CoStorage {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("CoStorage").finish()
	}
}
#[async_trait]
impl BlockStorage for CoStorage {
	/// Returns a block from storage.
	async fn get(&self, cid: &Cid) -> Result<Block, StorageError> {
		self.inner.get(cid).await
	}

	/// Inserts a block into storage.
	/// Returns the CID of the block (gurranted to be the same as the supplied).
	async fn set(&self, block: Block) -> Result<Cid, StorageError> {
		self.inner.set(block).await
	}

	/// Remove a block.
	async fn remove(&self, cid: &Cid) -> Result<(), StorageError> {
		self.inner.remove(cid).await
	}

	/// Stat a block.
	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError> {
		self.inner.stat(cid).await
	}

	/// Maximum accepted block size.
	fn max_block_size(&self) -> usize {
		self.inner.max_block_size()
	}
}
#[async_trait]
impl ExtendedBlockStorage for CoStorage {
	async fn set_extended(&self, block: ExtendedBlock) -> Result<Cid, StorageError> {
		self.inner.set_extended(block).await
	}

	async fn exists(&self, cid: &Cid) -> Result<bool, StorageError> {
		self.inner.exists(cid).await
	}

	async fn clear(&self) -> Result<(), StorageError> {
		self.inner.clear().await
	}
}
impl CloneWithBlockStorageSettings for CoStorage {
	fn clone_with_settings(&self, settings: BlockStorageCloneSettings) -> Self {
		CoStorage { inner: self.inner.clone_arc_with_settings(settings) }
	}
}
#[async_trait]
impl BlockStorageContentMapping for CoStorage {
	async fn is_content_mapped(&self) -> bool {
		self.inner.is_content_mapped().await
	}

	async fn to_plain(&self, mapped: &Cid) -> Option<Cid> {
		self.inner.to_plain(mapped).await
	}

	async fn to_mapped(&self, plain: &Cid) -> Option<Cid> {
		self.inner.to_mapped(plain).await
	}

	async fn insert_mappings(&self, mappings: BTreeSet<MappedCid>) {
		self.inner.insert_mappings(mappings).await
	}
}

trait CoStorageBlockStorage:
	BlockStorage + ExtendedBlockStorage + BlockStorageContentMapping + CloneArcWithSettings
{
}
impl<T> CoStorageBlockStorage for T where
	T: BlockStorage + ExtendedBlockStorage + BlockStorageContentMapping + CloneArcWithSettings
{
}

trait CloneArcWithSettings {
	fn clone_arc_with_settings(&self, settings: BlockStorageCloneSettings) -> Arc<dyn CoStorageBlockStorage + 'static>;
}
impl<T> CloneArcWithSettings for T
where
	T: BlockStorage + ExtendedBlockStorage + BlockStorageContentMapping + CloneWithBlockStorageSettings + 'static,
{
	fn clone_arc_with_settings(&self, settings: BlockStorageCloneSettings) -> Arc<dyn CoStorageBlockStorage + 'static> {
		Arc::new(self.clone_with_settings(settings))
	}
}
