use crate::StorageError;
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{Block, StoreParams};

#[async_trait]
pub trait BlockStorage {
	type StoreParams: StoreParams;

	/// Returns a block from storage.
	async fn get(&self, cid: &Cid) -> Result<Block<Self::StoreParams>, StorageError>;

	/// Inserts a block into storage.
	/// Returns the CID of the block (gurranted to be the same as the supplied).
	async fn set(&self, block: Block<Self::StoreParams>) -> Result<Cid, StorageError>;

	/// Remove a block.
	async fn remove(&self, cid: &Cid) -> Result<(), StorageError>;

	/// Stat a block.
	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError>;
}

#[derive(Debug, Clone)]
pub struct BlockStat {
	pub size: u64,
}
