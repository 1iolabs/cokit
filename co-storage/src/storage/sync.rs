use crate::{BlockStat, BlockStorage, Storage, StorageError};
use async_trait::async_trait;
use futures::Future;
use libipld::{Block, Cid, DefaultParams};
use std::{
	sync::{
		mpsc::{SendError, Sender},
		Arc,
	},
	thread::{self, JoinHandle},
};
use tokio::runtime::Handle;

#[derive(Debug, Clone)]
pub struct SyncStorage {
	sender: Sender<Message>,
	_handle: Arc<JoinHandle<()>>,
}

impl SyncStorage {
	/// Construct threaded storage with next as underlying storage.
	pub fn new<T>(mut next: T) -> Self
	where
		T: Storage + Send + 'static,
	{
		let (sender, receiver) = std::sync::mpsc::channel::<Message>();
		let handle = thread::spawn(move || {
			fn handle_send_result<T>(t: Result<(), SendError<T>>) {
				if t.is_err() {
					// TODO: add log?
				}
			}
			loop {
				match receiver.recv() {
					Err(_) => break, // sender dropped
					Ok(Message::Get(cid, result)) => handle_send_result(result.send(next.get(&cid))),
					Ok(Message::Set(block, result)) => handle_send_result(result.send(next.set(block))),
					Ok(Message::Remove(cid, result)) => handle_send_result(result.send(next.remove(&cid))),
				}
			}
		});
		Self { sender, _handle: Arc::new(handle) }
	}
}

impl Storage for SyncStorage {
	fn get(&self, cid: &libipld::Cid) -> Result<Block<DefaultParams>, StorageError> {
		let (sender, receiver) = std::sync::mpsc::channel::<Result<Block<DefaultParams>, StorageError>>();
		self.sender
			.send(Message::Get(cid.clone(), sender))
			.map_err(|e| StorageError::Internal(e.into()))?;
		let result = receiver.recv();
		match result {
			Ok(e) => e,
			Err(e) => Err(StorageError::Internal(e.into())),
		}
	}

	fn set(&mut self, block: Block<DefaultParams>) -> Result<(), StorageError> {
		let (sender, receiver) = std::sync::mpsc::channel::<Result<(), StorageError>>();
		self.sender
			.send(Message::Set(block, sender))
			.map_err(|e| StorageError::Internal(e.into()))?;
		let result = receiver.recv();
		match result {
			Ok(e) => e,
			Err(e) => Err(StorageError::Internal(e.into())),
		}
	}

	fn remove(&mut self, cid: &Cid) -> Result<(), StorageError> {
		let (sender, receiver) = std::sync::mpsc::channel::<Result<(), StorageError>>();
		self.sender
			.send(Message::Remove(cid.clone(), sender))
			.map_err(|e| StorageError::Internal(e.into()))?;
		let result = receiver.recv();
		match result {
			Ok(e) => e,
			Err(e) => Err(StorageError::Internal(e.into())),
		}
	}
}

#[derive(Debug)]
enum Message {
	Get(Cid, Sender<Result<Block<DefaultParams>, StorageError>>),
	Set(Block<DefaultParams>, Sender<Result<(), StorageError>>),
	Remove(Cid, Sender<Result<(), StorageError>>),
}

#[derive(Debug, Clone)]
pub struct SyncBlockStorage<S> {
	storage: S,
	handle: Handle,
}
impl<S> SyncBlockStorage<S>
where
	S: BlockStorage + Send + Sync + Clone + 'static,
{
	pub fn new(storage: S, handle: Handle) -> Self {
		Self { storage, handle }
	}

	fn execute<F, R>(&self, f: F) -> Result<R, StorageError>
	where
		F: Future<Output = Result<R, StorageError>> + Send + 'static,
		F::Output: Send + 'static,
		R: Send + 'static,
	{
		match self.handle.block_on(self.handle.spawn(f)) {
			Ok(r) => r,
			Err(e) => Err(StorageError::Internal(e.into())),
		}
	}
}
impl<S> Storage for SyncBlockStorage<S>
where
	S: BlockStorage<StoreParams = DefaultParams> + Send + Sync + Clone + 'static,
{
	fn get(&self, cid: &Cid) -> Result<Block<DefaultParams>, StorageError> {
		let storage = self.storage.clone();
		let cid = cid.clone();
		self.execute(async move { storage.get(&cid).await })
	}

	fn set(&mut self, block: Block<DefaultParams>) -> Result<(), StorageError> {
		let storage = self.storage.clone();
		self.execute(async move {
			storage.set(block).await?;
			Ok(())
		})
	}

	fn remove(&mut self, cid: &Cid) -> Result<(), StorageError> {
		let storage = self.storage.clone();
		let cid = cid.clone();
		self.execute(async move { storage.remove(&cid).await })
	}
}
#[async_trait]
impl<S> BlockStorage for SyncBlockStorage<S>
where
	S: BlockStorage + Send + Sync + Clone + 'static,
{
	type StoreParams = S::StoreParams;

	/// Returns a block from storage.
	async fn get(&self, cid: &Cid) -> Result<Block<Self::StoreParams>, StorageError> {
		self.storage.get(cid).await
	}

	/// Inserts a block into storage.
	/// Returns the CID of the block (gurranted to be the same as the supplied).
	async fn set(&self, block: Block<Self::StoreParams>) -> Result<Cid, StorageError> {
		self.storage.set(block).await
	}

	/// Remove a block.
	async fn remove(&self, cid: &Cid) -> Result<(), StorageError> {
		self.storage.remove(cid).await
	}

	/// Stat a block.
	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError> {
		self.storage.stat(cid).await
	}
}
