use crate::{BlockStat, BlockStorage, StorageError};
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{Block, BlockStorageSettings, CloneWithBlockStorageSettings, StoreParams};
use futures::{
	channel::{mpsc, oneshot},
	SinkExt,
};

#[derive(Debug)]
pub enum Request<P: StoreParams> {
	Get(Cid, oneshot::Sender<Result<Block<P>, StorageError>>),
	Set(Block<P>, oneshot::Sender<Result<Cid, StorageError>>),
	Remove(Cid, oneshot::Sender<Result<(), StorageError>>),
	Stat(Cid, oneshot::Sender<Result<BlockStat, StorageError>>),
}

#[derive(Debug, Clone)]
pub struct RequestBlockStorage<S>
where
	S: BlockStorage + Send + Sync + Clone + 'static,
{
	sender: mpsc::Sender<Request<S::StoreParams>>,
}
impl<S> RequestBlockStorage<S>
where
	S: BlockStorage + Send + Sync + Clone + 'static,
{
	pub fn new(buffer: usize) -> (Self, mpsc::Receiver<Request<S::StoreParams>>) {
		let (tx, rx) = mpsc::channel(buffer);
		(Self { sender: tx }, rx)
	}
}
#[async_trait]
impl<S> BlockStorage for RequestBlockStorage<S>
where
	S: BlockStorage + Send + Sync + Clone + 'static,
{
	type StoreParams = S::StoreParams;

	/// Returns a block from storage.
	async fn get(&self, cid: &Cid) -> Result<Block<Self::StoreParams>, StorageError> {
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
	async fn set(&self, block: Block<Self::StoreParams>) -> Result<Cid, StorageError> {
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
}
impl<S> CloneWithBlockStorageSettings for RequestBlockStorage<S>
where
	S: BlockStorage + CloneWithBlockStorageSettings + 'static,
{
	fn clone_with_settings(&self, _settings: BlockStorageSettings) -> Self {
		self.clone()
	}
}
