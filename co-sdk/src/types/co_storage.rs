use async_trait::async_trait;
use cid::Cid;
use co_primitives::{Block, BlockStorageSettings, CloneWithBlockStorageSettings, DefaultParams};
use co_storage::{BlockStat, BlockStorage, BlockStorageContentMapping, StorageError};
use std::{fmt::Debug, sync::Arc};

/// Public storage API.
#[derive(Clone)]
pub struct CoStorage {
	inner: Arc<dyn CoStorageBlockStorage<StoreParams = DefaultParams>>,
}
impl CoStorage {
	pub fn new<S>(storage: S) -> Self
	where
		S: BlockStorage<StoreParams = DefaultParams> + CloneWithBlockStorageSettings + 'static,
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
impl CloneWithBlockStorageSettings for CoStorage {
	fn clone_with_settings(&self, settings: BlockStorageSettings) -> Self {
		CoStorage { inner: self.inner.clone_arc_with_settings(settings) }
	}
}

trait CoStorageBlockStorage: BlockStorage + CloneArcWithSettings {}
impl<T> CoStorageBlockStorage for T where T: BlockStorage + CloneArcWithSettings {}

trait CloneArcWithSettings {
	fn clone_arc_with_settings(
		&self,
		settings: BlockStorageSettings,
	) -> Arc<dyn CoStorageBlockStorage<StoreParams = DefaultParams> + 'static>;
}
impl<T> CloneArcWithSettings for T
where
	T: BlockStorage<StoreParams = DefaultParams> + CloneWithBlockStorageSettings + 'static,
{
	fn clone_arc_with_settings(
		&self,
		settings: BlockStorageSettings,
	) -> Arc<dyn CoStorageBlockStorage<StoreParams = DefaultParams> + 'static> {
		Arc::new(self.clone_with_settings(settings))
	}
}

// #[async_trait]
// impl TransactionBlockStorage for CoStorage {
// 	// async fn flush(&self) -> Result<(), StorageError> {
// 	// 	self.inner.flush().await
// 	// }

// 	fn transaction(
// 		&self,
// 		settings: TransactionBlockStorageSettings,
// 	) -> Arc<(dyn TransactionBlockStorage<StoreParams = DefaultParams>)> {
// 		self.inner.transaction(settings)
// 	}
// }

#[derive(Clone)]
pub struct CoBlockStorageContentMapping {
	inner: Arc<dyn BlockStorageContentMapping + Send + Sync + 'static>,
}
impl CoBlockStorageContentMapping {
	pub fn new<M>(mapping: M) -> Self
	where
		M: BlockStorageContentMapping + Send + Sync + 'static,
	{
		Self { inner: Arc::new(mapping) }
	}
}
#[async_trait]
impl BlockStorageContentMapping for CoBlockStorageContentMapping {
	async fn to_plain(&self, mapped: &Cid) -> Option<Cid> {
		self.inner.to_plain(mapped).await
	}

	async fn to_mapped(&self, plain: &Cid) -> Option<Cid> {
		self.inner.to_mapped(plain).await
	}
}
