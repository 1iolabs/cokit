use crate::{AnyBlockStorage, Block, BlockStorage, BlockStorageStoreParams, DefaultParams, StorageError, StoreParams};
use anyhow::anyhow;
use async_trait::async_trait;
use cid::Cid;
use std::{fmt::Debug, sync::Arc};

#[derive(Clone)]
pub struct CoreBlockStorage {
	checked: bool,
	next: Arc<dyn BlockStorage + 'static>,
}
impl CoreBlockStorage {
	pub fn new(storage: impl AnyBlockStorage, checked: bool) -> Self {
		Self { checked, next: Arc::new(storage) }
	}
}
impl Debug for CoreBlockStorage {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("CoreBlockStorage").field("checked", &self.checked).finish()
	}
}
#[async_trait]
impl BlockStorage for CoreBlockStorage {
	/// Returns a block from storage.
	async fn get(&self, cid: &Cid) -> Result<Block, StorageError> {
		self.next.get(cid).await
	}

	/// Inserts a block into storage.
	async fn set(&self, block: Block) -> Result<Cid, StorageError> {
		self.next
			.set(if self.checked {
				block
					.with_store_params::<<Self as BlockStorageStoreParams>::StoreParams>()?
					.with_verify()?
			} else {
				block.with_store_params::<<Self as BlockStorageStoreParams>::StoreParams>()?
			})
			.await
	}

	/// Remove a block.
	async fn remove(&self, _cid: &Cid) -> Result<(), StorageError> {
		Err(StorageError::Internal(anyhow!("Unsupported in cores")))
	}

	/// Maximum accepted block size.
	fn max_block_size(&self) -> usize {
		<Self as BlockStorageStoreParams>::StoreParams::MAX_BLOCK_SIZE
	}
}
/// We do not want dynamic block limits in cores as this breaks determinism.
/// Force all blocks created on cores to be max. 1MiB.
impl BlockStorageStoreParams for CoreBlockStorage {
	type StoreParams = DefaultParams;
}
