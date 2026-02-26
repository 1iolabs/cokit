// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::library::co_actor::CoMessage;
use async_trait::async_trait;
use cid::Cid;
use co_actor::ActorHandle;
use co_primitives::{Block, BlockStorageCloneSettings, CloneWithBlockStorageSettings, StoreParams};
use co_sdk::{BlockStat, BlockStorage, BlockStorageContentMapping, DefaultParams, StorageError};
#[cfg(feature = "js")]
use tokio_with_wasm::alias as tokio;

#[derive(Debug, Clone)]
pub struct CoBlockStorage {
	settings: Option<BlockStorageCloneSettings>,
	handle: ActorHandle<CoMessage>,
}
impl CoBlockStorage {
	pub(crate) fn new(handle: ActorHandle<CoMessage>, settings: Option<BlockStorageCloneSettings>) -> Self {
		Self { settings, handle }
	}
}
#[async_trait]
impl BlockStorage for CoBlockStorage {
	/// Returns a block from storage.
	async fn get(&self, cid: &Cid) -> Result<Block, StorageError> {
		self.handle
			.request(|response| CoMessage::BlockGet(*cid, self.settings.clone(), response))
			.await
			.map_err(|err| StorageError::Internal(err.into()))?
	}

	/// Inserts a block into storage.
	/// Returns the CID of the block (gurranted to be the same as the supplied).
	async fn set(&self, block: Block) -> Result<Cid, StorageError> {
		self.handle
			.request(|response| CoMessage::BlockSet(block, self.settings.clone(), response))
			.await
			.map_err(|err| StorageError::Internal(err.into()))?
	}

	/// Remove a block.
	async fn remove(&self, cid: &Cid) -> Result<(), StorageError> {
		self.handle
			.request(|response| CoMessage::BlockRemove(*cid, self.settings.clone(), response))
			.await
			.map_err(|err| StorageError::Internal(err.into()))?
	}

	/// Stat a block.
	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError> {
		self.handle
			.request(|response| CoMessage::BlockStat(*cid, self.settings.clone(), response))
			.await
			.map_err(|err| StorageError::Internal(err.into()))?
	}

	fn max_block_size(&self) -> usize {
		DefaultParams::MAX_BLOCK_SIZE
	}
}
impl CloneWithBlockStorageSettings for CoBlockStorage {
	fn clone_with_settings(&self, settings: BlockStorageCloneSettings) -> Self {
		CoBlockStorage { settings: Some(settings), handle: self.handle.clone() }
	}
}
impl BlockStorageContentMapping for CoBlockStorage {}
