use libipld::{Block, Cid, DefaultParams};

/// Storage interface.
pub trait Storage {
	/// Returns a block from storage.
	fn get(&self, cid: &Cid) -> Result<Block<DefaultParams>, StorageError>;

	/// Inserts a block into storage.
	fn set(&mut self, block: Block<DefaultParams>) -> Result<(), StorageError>;
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
	#[error("Block not found")]
	NotFound,

	#[error("Internal storage error")]
	Internal,

	#[error("Invalid argument")]
	InvalidArgument,
}

/// Async storage interface.
#[async_trait::async_trait]
pub trait AsyncStorage {
	/// Returns a block from storage.
	async fn get(&self, cid: &Cid) -> Result<Block<DefaultParams>, StorageError>;

	/// Inserts a block into storage.
	async fn set(&mut self, block: Block<DefaultParams>) -> Result<(), StorageError>;
}
