use co_storage::{Storage, StorageError};
use libipld::Cid;

use super::entry::EntryBlock;

pub trait TypedStorage<T> {
	/// Returns a block from storage.
	fn get(&self, cid: &Cid) -> Result<T, StorageError>;

	/// Inserts a block into storage.
	fn set(&mut self, block: T) -> Result<(), StorageError>;
}

pub struct EntryStorage {
	next: Box<dyn Storage>,
}
impl EntryStorage {
	pub fn new(next: Box<dyn Storage>) -> Self {
		Self { next }
	}
}
impl TypedStorage<EntryBlock> for EntryStorage {
	fn get(&self, cid: &Cid) -> Result<EntryBlock, StorageError> {
		EntryBlock::from_signed_block(self.next.get(cid)?).map_err(|e| StorageError::InvalidArgument)
	}

	fn set(&mut self, block: EntryBlock) -> Result<(), StorageError> {
		match block.signed_block() {
			None => Err(StorageError::InvalidArgument),
			Some(Ok(block)) => self.next.set(block),
			Some(Err(e)) => Err(StorageError::InvalidArgument),
		}
	}
}
