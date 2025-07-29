use crate::{Block, BlockSerializerError, CborError, JsonError, MultiCodecError, StoreParams};
use async_trait::async_trait;
use cid::Cid;
use std::num::TryFromIntError;

#[async_trait]
pub trait BlockStorage: Send + Sync {
	type StoreParams: StoreParams;

	/// Returns a block from storage.
	async fn get(&self, cid: &Cid) -> Result<Block<Self::StoreParams>, StorageError>;

	/// Inserts a block into storage.
	async fn set(&self, block: Block<Self::StoreParams>) -> Result<Cid, StorageError>;

	/// Stat a block.
	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError> {
		let block = self.get(cid).await?;
		Ok(BlockStat {
			size: block
				.data()
				.len()
				.try_into()
				.map_err(|e: TryFromIntError| StorageError::Internal(e.into()))?,
		})
	}

	/// Remove a block.
	async fn remove(&self, cid: &Cid) -> Result<(), StorageError>;
}

pub trait CloneWithBlockStorageSettings: Clone {
	fn clone_with_settings(&self, settings: BlockStorageSettings) -> Self;

	fn without_networking(&self) -> Self {
		self.clone_with_settings(BlockStorageSettings::new().without_networking())
	}
}

#[derive(Debug, Clone, Default)]
pub struct BlockStorageSettings {
	/// Do not allow to use networking in the clone.
	pub disallow_networking: bool,

	/// Detach as child instance from the parent.
	pub detached: bool,

	/// Clone with empty caches.
	pub clear: bool,
}
impl BlockStorageSettings {
	pub fn new() -> Self {
		Default::default()
	}

	pub fn without_networking(mut self) -> Self {
		self.disallow_networking = true;
		self
	}

	pub fn with_detached(mut self) -> Self {
		self.detached = true;
		self
	}

	pub fn with_clear(mut self) -> Self {
		self.clear = true;
		self
	}
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlockStat {
	pub size: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
	/// Block not found error.
	/// This error is may be temporarily as the block may comes available on the network.
	#[error("Block not found: {0}")]
	NotFound(Cid, #[source] anyhow::Error),

	/// Internal storage error.
	/// This indicates some invalid state and is not be retriable with same parameters.
	#[error("Internal storage error")]
	Internal(#[from] anyhow::Error),

	/// Invalid argument passes to call or storage configuration.
	/// This is not be retriable with same parameters.
	#[error("Invalid storage argument")]
	InvalidArgument(#[source] anyhow::Error),
}
impl Clone for StorageError {
	/// Clone the StorageError.
	/// Note: The clone will only clone a Debug representation of the source error.
	fn clone(&self) -> Self {
		match self {
			StorageError::NotFound(cid, error) => StorageError::NotFound(*cid, anyhow::anyhow!("{:?}", error)),
			StorageError::Internal(error) => StorageError::Internal(anyhow::anyhow!("{:?}", error)),
			StorageError::InvalidArgument(error) => StorageError::InvalidArgument(anyhow::anyhow!("{:?}", error)),
		}
	}
}
impl From<MultiCodecError> for StorageError {
	fn from(value: MultiCodecError) -> Self {
		StorageError::InvalidArgument(value.into())
	}
}
impl From<BlockSerializerError> for StorageError {
	fn from(value: BlockSerializerError) -> Self {
		StorageError::InvalidArgument(value.into())
	}
}
impl From<CborError> for StorageError {
	fn from(value: CborError) -> Self {
		StorageError::InvalidArgument(value.into())
	}
}
impl From<JsonError> for StorageError {
	fn from(value: JsonError) -> Self {
		StorageError::InvalidArgument(value.into())
	}
}
