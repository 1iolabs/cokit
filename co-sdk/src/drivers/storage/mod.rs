use crate::CoStorage;
use co_storage::{FsStorage, MemoryBlockStorage};
use std::path::PathBuf;

#[derive(Clone)]
pub struct Storage {
	storage: CoStorage,
}
impl Storage {
	pub fn new(storage_path: PathBuf) -> Self {
		Self { storage: CoStorage::new(FsStorage::new(storage_path)) }
	}

	pub fn new_memory() -> Self {
		Self { storage: CoStorage::new(MemoryBlockStorage::new()) }
	}

	pub fn storage(&self) -> CoStorage {
		self.storage.clone()
	}
}
