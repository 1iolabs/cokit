use cid::Cid;
use co_primitives::{Block, StorageError, StoreParams};

/// Storage interface.
pub trait Storage {
	type StoreParams: StoreParams;

	/// Returns a block from storage.
	fn get(&self, cid: &Cid) -> Result<Block<Self::StoreParams>, StorageError>;

	/// Inserts a block into storage.
	fn set(&mut self, block: Block<Self::StoreParams>) -> Result<Cid, StorageError>;

	/// Remove a block from storage.
	fn remove(&mut self, cid: &Cid) -> Result<(), StorageError>;
}
