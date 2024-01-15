use crate::{Storage, StorageError};
use libipld::{Block, Cid, DefaultParams};
use std::{
	sync::mpsc::{SendError, Sender},
	thread::{self, JoinHandle},
};

#[derive(Clone)]
pub struct SyncStorage {
	sender: Sender<Message>,
}

impl SyncStorage {
	/// Construct threaded storage with next as underlying storage.
	pub fn new<T>(mut next: T) -> (Self, JoinHandle<()>)
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
				}
			}
		});
		(Self { sender }, handle)
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
}

#[derive(Debug)]
enum Message {
	Get(Cid, Sender<Result<Block<DefaultParams>, StorageError>>),
	Set(Block<DefaultParams>, Sender<Result<(), StorageError>>),
}
