use crate::CoStorage;
use co_storage::FsStorage;
use std::{path::PathBuf, sync::Arc};

pub struct Storage {
	storage: CoStorage,
}
impl Storage {
	pub fn new(storage_path: PathBuf) -> Self {
		Self { storage: CoStorage::new(Arc::new(FsStorage::new(storage_path))) }
	}

	pub fn storage(&self) -> CoStorage {
		self.storage.clone()
	}
}
