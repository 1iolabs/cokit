use crate::StorageError;
use async_trait::async_trait;
use libipld::{store::StoreParams, Block, Cid};

#[async_trait(?Send)]
pub trait BlockStorage {
	type StoreParams: StoreParams;

	/// Returns a block from storage.
	async fn get(&self, cid: &Cid) -> Result<Block<Self::StoreParams>, StorageError>;

	/// Inserts a block into storage.
	async fn set(&mut self, block: Block<Self::StoreParams>) -> Result<(), StorageError>;

	/// Remove a block.
	async fn remove(&mut self, cid: &Cid) -> Result<(), StorageError>;

	/// Stat a block.
	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError>;
}

#[derive(Debug, Clone)]
pub struct BlockStat {
	pub size: u64,
}
