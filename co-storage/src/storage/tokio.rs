use crate::{BlockStat, BlockStorage, StorageError};
use async_trait::async_trait;
use libipld::{store::StoreParams, Block, Cid};
use std::sync::Arc;
use tokio::{
	sync::{
		mpsc::{unbounded_channel, UnboundedSender},
		oneshot::{self},
	},
	task::JoinHandle,
};

#[derive(Clone)]
pub struct SyncBlockStorage<P: StoreParams> {
	sender: UnboundedSender<Message<P>>,
	_handle: Arc<JoinHandle<()>>,
}

impl<P: StoreParams> SyncBlockStorage<P> {
	/// Construct threaded storage with next as underlying storage.
	pub fn new<T>(mut next: T) -> Self
	where
		T: BlockStorage<StoreParams = P> + Send + 'static,
	{
		let (sender, mut receiver) = unbounded_channel::<Message<P>>();
		let handle = tokio::task::spawn_local(async move {
			fn handle_send_result<T>(t: Result<(), T>) {
				if t.is_err() {
					// TODO: add log?
				}
			}
			loop {
				match receiver.recv().await {
					None => break, // sender dropped
					Some(Message::Get(cid, result)) => handle_send_result(result.send(next.get(&cid).await)),
					Some(Message::Set(block, result)) => handle_send_result(result.send(next.set(block).await)),
					Some(Message::Remove(cid, result)) => handle_send_result(result.send(next.remove(&cid).await)),
					Some(Message::Stat(cid, result)) => handle_send_result(result.send(next.stat(&cid).await)),
				}
			}
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
			.send(Message::Get(cid.clone(), sender))
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
			.send(Message::Stat(cid.clone(), sender))
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
