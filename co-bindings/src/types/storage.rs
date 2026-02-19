// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{CoCid, CoError};
use async_trait::async_trait;
use std::sync::Arc;

#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
#[cfg_attr(feature = "frb", flutter_rust_bridge::frb(opaque))]
#[derive(Clone)]
pub struct BlockStorage {
	storage: Arc<dyn co_sdk::BlockStorage + 'static>,
}
impl BlockStorage {
	#[cfg_attr(feature = "frb", flutter_rust_bridge::frb(ignore))]
	pub fn new(storage: impl co_sdk::BlockStorage + 'static) -> Self {
		Self { storage: Arc::new(storage) }
	}

	#[cfg_attr(feature = "frb", flutter_rust_bridge::frb(name = "getBlock"))]
	pub async fn get(&self, cid: &CoCid) -> Result<Block, CoError> {
		Ok(self.storage.get(&cid.cid()?).await.map_err(CoError::new)?.into())
	}

	#[cfg_attr(feature = "frb", flutter_rust_bridge::frb(name = "setBlock"))]
	pub async fn set(&self, block: Block) -> Result<CoCid, CoError> {
		Ok(self.storage.set(block.try_into()?).await.map_err(CoError::new)?.into())
	}
}
#[cfg_attr(feature = "frb", flutter_rust_bridge::frb(ignore))]
#[async_trait]
impl co_sdk::BlockStorage for BlockStorage {
	async fn get(&self, cid: &cid::Cid) -> Result<co_primitives::Block, co_primitives::StorageError> {
		Ok(self.storage.get(cid).await?)
	}

	async fn set(&self, block: co_primitives::Block) -> Result<cid::Cid, co_primitives::StorageError> {
		Ok(self.storage.set(block).await?)
	}

	async fn remove(&self, cid: &cid::Cid) -> Result<(), co_primitives::StorageError> {
		Ok(self.storage.remove(cid).await?)
	}

	fn max_block_size(&self) -> usize {
		self.storage.max_block_size()
	}
}

#[derive(Debug, Clone)]
pub struct Block {
	pub cid: CoCid,
	pub data: Vec<u8>,
}
impl Block {
	/// Creates a new block. Returns an error if the hash doesn't match
	/// the data.
	pub fn new(cid: CoCid, data: Vec<u8>) -> Result<Self, CoError> {
		let (cid, data) = co_sdk::Block::new(cid.cid()?, data).map_err(CoError::new)?.into_inner();
		Ok(Self { cid: cid.into(), data })
	}

	/// Creates a new block without verifying the cid.
	pub fn new_unchecked(cid: CoCid, data: Vec<u8>) -> Self {
		Self { cid, data }
	}

	/// Create a new block by calculating the [`Cid`] from data using the default hasher.
	/// Note: The default hasher may changes without notice.
	pub fn new_data(codec: u64, data: Vec<u8>) -> Self {
		Self::new_unchecked(co_sdk::Block::cid_data(codec, &data).into(), data)
	}
}
impl From<co_sdk::Block> for Block {
	fn from(value: co_sdk::Block) -> Self {
		let (cid, data) = value.into_inner();
		Self { cid: cid.into(), data }
	}
}
impl TryFrom<Block> for co_sdk::Block {
	type Error = CoError;

	fn try_from(value: Block) -> Result<Self, Self::Error> {
		Self::new(value.cid.cid()?, value.data).map_err(CoError::new)
	}
}
