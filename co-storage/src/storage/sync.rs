// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{BlockStat, BlockStorage, Storage, StorageError};
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{Block, DefaultParams};
use futures::Future;
use std::{
	marker::PhantomData,
	sync::{
		mpsc::{SendError, Sender},
		Arc,
	},
	thread::{self, JoinHandle},
};
use tokio::runtime::Handle;

#[derive(Debug)]
pub struct SyncStorage<S>
where
	S: Storage,
{
	sender: Sender<Message>,
	_handle: Arc<JoinHandle<()>>,
	_type: PhantomData<S>,
}
impl<S> Clone for SyncStorage<S>
where
	S: Storage,
{
	fn clone(&self) -> Self {
		Self { sender: self.sender.clone(), _handle: self._handle.clone(), _type: self._type }
	}
}
impl<S> SyncStorage<S>
where
	S: Storage + Send + 'static,
{
	/// Construct threaded storage with next as underlying storage.
	pub fn new(mut next: S) -> Self {
		let (sender, receiver) = std::sync::mpsc::channel::<Message>();
		let handle = thread::spawn(move || {
			fn handle_send_result<T>(t: Result<(), SendError<T>>) {
				if let Err(err) = t {
					tracing::warn!(?err, "send-failed");
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
		Self { sender, _handle: Arc::new(handle), _type: Default::default() }
	}
}
impl<S> Storage for SyncStorage<S>
where
	S: Storage + Send + 'static,
{
	type StoreParams = S::StoreParams;

	fn get(&self, cid: &Cid) -> Result<Block, StorageError> {
		let (sender, receiver) = std::sync::mpsc::channel::<Result<Block, StorageError>>();
		self.sender
			.send(Message::Get(*cid, sender))
			.map_err(|e| StorageError::Internal(e.into()))?;
		let result = receiver.recv();
		match result {
			Ok(e) => e,
			Err(e) => Err(StorageError::Internal(e.into())),
		}
	}

	fn set(&mut self, block: Block) -> Result<Cid, StorageError> {
		let (sender, receiver) = std::sync::mpsc::channel::<Result<Cid, StorageError>>();
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
			.send(Message::Remove(*cid, sender))
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
	Get(Cid, Sender<Result<Block, StorageError>>),
	Set(Block, Sender<Result<Cid, StorageError>>),
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
	S: BlockStorage + Send + Sync + Clone + 'static,
{
	type StoreParams = DefaultParams;

	fn get(&self, cid: &Cid) -> Result<Block, StorageError> {
		let storage = self.storage.clone();
		let cid = *cid;
		self.execute(async move { storage.get(&cid).await })
	}

	fn set(&mut self, block: Block) -> Result<Cid, StorageError> {
		let storage = self.storage.clone();
		self.execute(async move { storage.set(block).await })
	}

	fn remove(&mut self, cid: &Cid) -> Result<(), StorageError> {
		let storage = self.storage.clone();
		let cid = *cid;
		self.execute(async move { storage.remove(&cid).await })
	}
}
#[async_trait]
impl<S> BlockStorage for SyncBlockStorage<S>
where
	S: BlockStorage + Clone + 'static,
{
	/// Returns a block from storage.
	async fn get(&self, cid: &Cid) -> Result<Block, StorageError> {
		self.storage.get(cid).await
	}

	/// Inserts a block into storage.
	/// Returns the CID of the block (gurranted to be the same as the supplied).
	async fn set(&self, block: Block) -> Result<Cid, StorageError> {
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

	fn max_block_size(&self) -> usize {
		self.storage.max_block_size()
	}
}
