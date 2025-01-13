use cid::Cid;
use co_primitives::{Block, MultiCodecError, StoreParams};

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

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
	/// Block not found error.
	/// This error is may be temporarily as the block may comes availabvle on the network.
	#[error("Block not found: {0}")]
	NotFound(Cid, #[source] anyhow::Error),

	/// Internal storage error.
	/// This indicates some invalid state and is not be retriable with same parameters.
	#[error("Internal storage error")]
	Internal(#[from] anyhow::Error),

	/// Invalid argument passes to call or storage configuration.
	/// This is not be retriable with same parameters.
	#[error("Invalid argument")]
	InvalidArgument(#[source] anyhow::Error),
}
impl From<MultiCodecError> for StorageError {
	fn from(value: MultiCodecError) -> Self {
		StorageError::InvalidArgument(value.into())
	}
}
