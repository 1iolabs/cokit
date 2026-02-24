// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::CoStorage;
#[cfg(feature = "fs")]
use crate::CoUuid;
#[cfg(feature = "fs")]
use crate::DynamicCoUuid;
#[cfg(feature = "fs")]
use co_storage::FsStorage;
use co_storage::{
	Algorithm, EncryptedBlockStorage, EncryptionReferenceMode, JoinBlockStorage, MemoryBlockStorage, StaticBlockStorage,
};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Storage {
	storage: CoStorage,
	tmp_storage: TmpStorage,
}
impl Storage {
	#[cfg(feature = "fs")]
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

	pub fn with_static(mut self, storages: Vec<StaticBlockStorage<'static>>) -> Self {
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
			#[cfg(feature = "fs")]
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
	#[cfg(feature = "fs")]
	Path(DynamicCoUuid, PathBuf),
}
