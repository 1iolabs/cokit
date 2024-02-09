use async_trait::async_trait;
use co_storage::{BlockStat, BlockStorage, StorageError};
use libipld::{Block, Cid, DefaultParams};
use std::sync::Arc;

/// Public storage API.
#[derive(Clone)]
pub struct CoStorage {
	inner: Arc<dyn BlockStorage<StoreParams = DefaultParams> + Send + Sync>,
}
impl CoStorage {
	pub fn new<S>(storage: S) -> Self
	where
		S: BlockStorage<StoreParams = DefaultParams> + Send + Sync + 'static,
	{
		Self { inner: Arc::new(storage) }
	}
}
#[async_trait]
impl BlockStorage for CoStorage {
	type StoreParams = DefaultParams;

	/// Returns a block from storage.
	async fn get(&self, cid: &Cid) -> Result<Block<Self::StoreParams>, StorageError> {
		self.inner.get(cid).await
	}

	/// Inserts a block into storage.
	/// Returns the CID of the block (gurranted to be the same as the supplied).
	async fn set(&self, block: Block<Self::StoreParams>) -> Result<Cid, StorageError> {
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
}
