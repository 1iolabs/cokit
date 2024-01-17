use crate::{BlockStat, BlockStorage, StorageError};
use async_trait::async_trait;
use futures::{
	channel::{
		mpsc::{self},
		oneshot::{self},
	},
	SinkExt, StreamExt,
};
use libipld::{store::StoreParams, Block, Cid};
use std::sync::Arc;
use tokio::task::JoinHandle;

#[derive(Debug, Clone)]
pub struct SyncBlockStorage<P: StoreParams> {
	sender: mpsc::UnboundedSender<Message<P>>,
	_handle: Arc<JoinHandle<()>>,
}

impl<P: StoreParams> SyncBlockStorage<P> {
	/// Construct threaded storage with next as underlying storage.
	pub fn new<T>(mut next: T) -> Self
	where
		T: BlockStorage<StoreParams = P> + Send + 'static,
	{
		let (sender, mut receiver) = mpsc::unbounded::<Message<P>>();
		let runtime_handle = tokio::runtime::Handle::current();
		let handle = tokio::task::spawn_blocking(move || {
			runtime_handle.block_on(async move {
				fn handle_send_result<T>(t: Result<(), T>) {
					if t.is_err() {
						// TODO: add log?
					}
				}
				while let Some(message) = receiver.next().await {
					match message {
						Message::Get(cid, result) => handle_send_result(result.send(next.get(&cid).await)),
						Message::Set(block, result) => handle_send_result(result.send(next.set(block).await)),
						Message::Remove(cid, result) => handle_send_result(result.send(next.remove(&cid).await)),
						Message::Stat(cid, result) => handle_send_result(result.send(next.stat(&cid).await)),
					}
				}
			});
		});
		Self { sender, _handle: Arc::new(handle) }
	}
}

#[async_trait(?Send)]
impl<P: StoreParams> BlockStorage for SyncBlockStorage<P> {
	type StoreParams = P;

	async fn get(&self, cid: &Cid) -> Result<Block<Self::StoreParams>, StorageError> {
		let (sender, receiver) = oneshot::channel();
		self.sender
			.clone()
			.send(Message::Get(cid.clone(), sender))
			.await
			.map_err(|e| StorageError::Internal(e.into()))?;
		let result = receiver.await;
		match result {
			Ok(e) => e,
			Err(e) => Err(StorageError::Internal(e.into())),
		}
	}

	async fn set(&mut self, block: Block<Self::StoreParams>) -> Result<(), StorageError> {
		let (sender, receiver) = oneshot::channel();
		self.sender
			.send(Message::Set(block, sender))
			.await
			.map_err(|e| StorageError::Internal(e.into()))?;
		let result = receiver.await;
		match result {
			Ok(e) => e,
			Err(e) => Err(StorageError::Internal(e.into())),
		}
	}

	async fn remove(&mut self, cid: &Cid) -> Result<(), StorageError> {
		let (sender, receiver) = oneshot::channel();
		self.sender
			.send(Message::Remove(cid.clone(), sender))
			.await
			.map_err(|e| StorageError::Internal(e.into()))?;
		let result = receiver.await;
		match result {
			Ok(e) => e,
			Err(e) => Err(StorageError::Internal(e.into())),
		}
	}

	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError> {
		let (sender, receiver) = oneshot::channel();
		self.sender
			.clone()
			.send(Message::Stat(cid.clone(), sender))
			.await
			.map_err(|e| StorageError::Internal(e.into()))?;
		let result = receiver.await;
		match result {
			Ok(e) => e,
			Err(e) => Err(StorageError::Internal(e.into())),
		}
	}
}

#[derive(Debug)]
enum Message<P> {
	Get(Cid, oneshot::Sender<Result<Block<P>, StorageError>>),
	Set(Block<P>, oneshot::Sender<Result<(), StorageError>>),
	Remove(Cid, oneshot::Sender<Result<(), StorageError>>),
	Stat(Cid, oneshot::Sender<Result<BlockStat, StorageError>>),
}

#[cfg(test)]
mod tests {
	use crate::{BlockSerializer, BlockStorage, MemoryStorage, SyncBlockStorage};

	#[tokio::test]
	async fn smoke() {
		let memory = MemoryStorage::new();
		let mut sync = SyncBlockStorage::new(memory);
		let block = BlockSerializer::default().serialize(&123).unwrap();
		sync.set(block.clone()).await.unwrap();
		let block_get = sync.get(block.cid()).await.unwrap();
		assert_eq!(block_get, block);
	}
}
