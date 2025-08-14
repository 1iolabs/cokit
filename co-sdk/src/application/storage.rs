use crate::{CoStorage, CoUuid, DynamicCoUuid};
use co_primitives::DefaultParams;
use co_storage::{
	Algorithm, EncryptedBlockStorage, EncryptionReferenceMode, FsStorage, JoinBlockStorage, MemoryBlockStorage,
	StaticBlockStorage,
};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Storage {
	storage: CoStorage,
	tmp_storage: TmpStorage,
}
impl Storage {
	pub fn new(storage_path: PathBuf, tmp_storage_path: PathBuf, tmp_uuid: DynamicCoUuid) -> Self {
		// to prevent data loss verify the tmp folder is not the actual storage folder
		assert!(storage_path != tmp_storage_path);

		// result
		Self {
			storage: CoStorage::new(FsStorage::new(storage_path)),
			tmp_storage: TmpStorage::Path(tmp_uuid, tmp_storage_path),
		}
	}

	pub fn new_memory() -> Self {
		Self { storage: CoStorage::new(MemoryBlockStorage::default()), tmp_storage: TmpStorage::Memory }
	}

	pub fn with_static(mut self, storages: Vec<StaticBlockStorage<'static, DefaultParams>>) -> Self {
		self.storage = CoStorage::new(JoinBlockStorage::new(self.storage, storages));
		self
	}

	pub fn storage(&self) -> CoStorage {
		self.storage.clone()
	}

	/// Create a new (distinct) tmp storage instance.
	pub fn tmp_storage(&self) -> CoStorage {
		match &self.tmp_storage {
			TmpStorage::Memory => CoStorage::new(MemoryBlockStorage::default()),
			TmpStorage::Path(uuid, path) => CoStorage::new(
				EncryptedBlockStorage::new(
					FsStorage::new(path.join(uuid.uuid())).with_allow_clear(true),
					Algorithm::default().generate_serect(),
					Algorithm::default(),
					Default::default(),
				)
				.with_encryption_reference_mode(EncryptionReferenceMode::AllowPlain),
			),
		}
	}
}

#[derive(Debug, Clone)]
enum TmpStorage {
	Memory,
	Path(DynamicCoUuid, PathBuf),
}
