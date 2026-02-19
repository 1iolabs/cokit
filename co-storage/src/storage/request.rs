// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{BlockStat, BlockStorage, StorageError};
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{Block, BlockStorageCloneSettings, CloneWithBlockStorageSettings};
use futures::{
	channel::{mpsc, oneshot},
	SinkExt,
};

#[derive(Debug)]
pub enum Request {
	Get(Cid, oneshot::Sender<Result<Block, StorageError>>),
	Set(Block, oneshot::Sender<Result<Cid, StorageError>>),
	Remove(Cid, oneshot::Sender<Result<(), StorageError>>),
	Stat(Cid, oneshot::Sender<Result<BlockStat, StorageError>>),
}

#[derive(Debug, Clone)]
pub struct RequestBlockStorage {
	sender: mpsc::Sender<Request>,
	max_block_size: usize,
}
impl RequestBlockStorage {
	pub fn new(buffer: usize, max_block_size: usize) -> (Self, mpsc::Receiver<Request>) {
		let (tx, rx) = mpsc::channel(buffer);
		(Self { sender: tx, max_block_size }, rx)
	}
}
#[async_trait]
impl BlockStorage for RequestBlockStorage {
	/// Returns a block from storage.
	async fn get(&self, cid: &Cid) -> Result<Block, StorageError> {
		let (tx, rx) = oneshot::channel();
		self.sender
			.clone()
			.send(Request::Get(*cid, tx))
			.await
			.map_err(|e| StorageError::Internal(e.into()))?;
		rx.await.map_err(|e| StorageError::Internal(e.into()))?
	}

	/// Inserts a block into storage.
	/// Returns the CID of the block (gurranted to be the same as the supplied).
	async fn set(&self, block: Block) -> Result<Cid, StorageError> {
		let (tx, rx) = oneshot::channel();
		self.sender
			.clone()
			.send(Request::Set(block, tx))
			.await
			.map_err(|e| StorageError::Internal(e.into()))?;
		rx.await.map_err(|e| StorageError::Internal(e.into()))?
	}

	/// Remove a block.
	async fn remove(&self, cid: &Cid) -> Result<(), StorageError> {
		let (tx, rx) = oneshot::channel();
		self.sender
			.clone()
			.send(Request::Remove(*cid, tx))
			.await
			.map_err(|e| StorageError::Internal(e.into()))?;
		rx.await.map_err(|e| StorageError::Internal(e.into()))?
	}

	/// Stat a block.
	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError> {
		let (tx, rx) = oneshot::channel();
		self.sender
			.clone()
			.send(Request::Stat(*cid, tx))
			.await
			.map_err(|e| StorageError::Internal(e.into()))?;
		rx.await.map_err(|e| StorageError::Internal(e.into()))?
	}

	fn max_block_size(&self) -> usize {
		self.max_block_size
	}
}
impl CloneWithBlockStorageSettings for RequestBlockStorage {
	fn clone_with_settings(&self, _settings: BlockStorageCloneSettings) -> Self {
		self.clone()
	}
}
