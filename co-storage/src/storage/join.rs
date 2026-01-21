use crate::{BlockStorageContentMapping, ExtendedBlock, ExtendedBlockStorage};
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{
	Block, BlockStat, BlockStorage, BlockStorageCloneSettings, BlockStorageStoreParams, CloneWithBlockStorageSettings,
	MappedCid, StorageError,
};
use std::{collections::BTreeSet, sync::Arc};

/// Joins multiple block storages together.
/// Write operations will be always delegated to the first storage (which passed to `new`).
/// Read operations starts with the last up to the first.
#[derive(Debug, Clone)]
pub struct JoinBlockStorage<S, R> {
	next: Arc<(S, Vec<R>)>,
}
impl<S, R> JoinBlockStorage<S, R>
where
	S: BlockStorage + 'static,
	R: ExtendedBlockStorage + 'static,
{
	pub fn new(next: S, read: Vec<R>) -> Self {
		Self { next: Arc::new((next, read)) }
	}
}
#[async_trait]
impl<S, R> BlockStorage for JoinBlockStorage<S, R>
where
	S: BlockStorage + 'static,
	R: ExtendedBlockStorage,
{
	async fn get(&self, cid: &Cid) -> Result<Block, StorageError> {
		for read in self.next.1.iter() {
			if read.exists(cid).await.unwrap_or(false) {
				return read.get(cid).await;
			}
		}
		self.next.0.get(cid).await
	}

	async fn set(&self, block: Block) -> Result<Cid, StorageError> {
		self.next.0.set(block).await
	}

	async fn remove(&self, cid: &Cid) -> Result<(), StorageError> {
		self.next.0.remove(cid).await
	}

	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError> {
		for read in self.next.1.iter() {
			if read.exists(cid).await.unwrap_or(false) {
				return read.stat(cid).await;
			}
		}
		self.next.0.stat(cid).await
	}

	fn max_block_size(&self) -> usize {
		self.next.0.max_block_size()
	}
}
#[async_trait]
impl<S, R> ExtendedBlockStorage for JoinBlockStorage<S, R>
where
	S: ExtendedBlockStorage + 'static,
	R: ExtendedBlockStorage + 'static,
{
	async fn set_extended(&self, block: ExtendedBlock) -> Result<Cid, StorageError> {
		self.next.0.set_extended(block).await
	}

	async fn exists(&self, cid: &Cid) -> Result<bool, StorageError> {
		for read in self.next.1.iter() {
			if read.exists(cid).await.unwrap_or(false) {
				return Ok(true);
			}
		}
		self.next.0.exists(cid).await
	}

	async fn clear(&self) -> Result<(), StorageError> {
		self.next.0.clear().await
	}
}
#[async_trait]
impl<S, R> BlockStorageContentMapping for JoinBlockStorage<S, R>
where
	S: BlockStorage + BlockStorageContentMapping + 'static,
	R: ExtendedBlockStorage + 'static,
{
	async fn is_content_mapped(&self) -> bool {
		self.next.0.is_content_mapped().await
	}

	async fn to_plain(&self, mapped: &Cid) -> Option<Cid> {
		self.next.0.to_plain(mapped).await
	}

	async fn to_mapped(&self, plain: &Cid) -> Option<Cid> {
		self.next.0.to_mapped(plain).await
	}

	async fn insert_mappings(&self, mappings: BTreeSet<MappedCid>) {
		self.next.0.insert_mappings(mappings).await
	}
}
#[async_trait]
impl<S, R> CloneWithBlockStorageSettings for JoinBlockStorage<S, R>
where
	S: BlockStorage + CloneWithBlockStorageSettings + 'static,
	R: ExtendedBlockStorage + Clone + 'static,
{
	fn clone_with_settings(&self, settings: BlockStorageCloneSettings) -> Self {
		Self::new(self.next.0.clone_with_settings(settings), self.next.1.clone())
	}
}
impl<S, R> BlockStorageStoreParams for JoinBlockStorage<S, R>
where
	S: BlockStorage + BlockStorageStoreParams + 'static,
	R: ExtendedBlockStorage + 'static,
{
	type StoreParams = S::StoreParams;
}
