use crate::CoStorage;
use co_storage::{FsStorage, MemoryBlockStorage};
use std::path::PathBuf;

#[derive(Clone)]
pub struct Storage {
	storage: CoStorage,
	tmp_storage: CoStorage,
}
impl Storage {
	pub fn new(storage_path: PathBuf, tmp_storage_path: PathBuf) -> Self {
		Self {
			storage: CoStorage::new(FsStorage::new(storage_path)),
			tmp_storage: CoStorage::new(FsStorage::new(tmp_storage_path)),
		}
	}

	pub fn new_memory() -> Self {
		Self {
			storage: CoStorage::new(MemoryBlockStorage::default()),
			tmp_storage: CoStorage::new(MemoryBlockStorage::default()),
		}
	}

	pub fn storage(&self) -> CoStorage {
		self.storage.clone()
	}

	pub fn tmp_storage(&self) -> CoStorage {
		self.tmp_storage.clone()
	}
}
