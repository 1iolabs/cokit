// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{
	library::fs_write::{fs_write_atomic, fs_write_atomic_blocking},
	BlockStorageContentMapping, ExtendedBlock, ExtendedBlockStorage, Storage,
};
use anyhow::anyhow;
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{
	Block, BlockStat, BlockStorage, BlockStorageCloneSettings, CloneWithBlockStorageSettings, DefaultParams,
	StorageError, StoreParams,
};
use std::{
	io::ErrorKind,
	os::unix::fs::MetadataExt,
	path::{Path, PathBuf},
};

/// Filesystem storage.
///
/// Creates one file per CID.
/// To ensure directories arend getting too many entries extra folders are created for the furst two bytes of the CID
/// digest.
#[derive(Debug, Clone)]
pub struct FsStorage {
	path: PathBuf,
	allow_clear: bool,
	max_block_size: usize,
}
impl FsStorage {
	pub fn new(path: PathBuf) -> Self {
		Self { path, allow_clear: false, max_block_size: DefaultParams::MAX_BLOCK_SIZE }
	}

	pub fn create(&self) -> std::io::Result<()> {
		std::fs::create_dir_all(&self.path)
	}

	pub fn with_allow_clear(mut self, allow_clear: bool) -> Self {
		self.allow_clear = allow_clear;
		self
	}
}
impl Storage for FsStorage {
	type StoreParams = DefaultParams;

	fn get(&self, cid: &Cid) -> Result<Block, StorageError> {
		let path = to_cid_path(&self.path, cid, "");
		into_block_result(cid, std::fs::read(path))
	}

	fn set(&mut self, block: Block) -> Result<Cid, StorageError> {
		let path = to_cid_path(&self.path, block.cid(), "");

		// exists?
		match std::fs::metadata(&path) {
			// run some validations and skip re-write
			Ok(m) => {
				if !m.is_file() {
					return Err(StorageError::Internal(anyhow!("Unexpected file type: {:?}", path)));
				}
				if m.len() != block.data().len() as u64 {
					return Err(StorageError::Internal(anyhow!(
						"Unexpected file size: {} != {}: {:?}",
						m.len(),
						block.data().len(),
						path,
					)));
				}
				return Ok(block.into_inner().0);
			},
			// continue with write
			Err(e) if e.kind() == ErrorKind::NotFound => {},
			// forward other errors (permission, ...)
			Err(e) => return Err(StorageError::Internal(e.into())),
		}

		// write
		fs_write_atomic_blocking(&path, block.data(), true).map_err(|e| StorageError::Internal(e.into()))?;

		// result
		Ok(block.into_inner().0)
	}

	fn remove(&mut self, cid: &Cid) -> Result<(), StorageError> {
		let path = to_cid_path(&self.path, cid, "");
		into_storage_result(cid, std::fs::remove_file(path))
	}
}

#[async_trait]
impl BlockStorage for FsStorage {
	async fn get(&self, cid: &Cid) -> Result<Block, StorageError> {
		let path = to_cid_path(&self.path, cid, "");
		into_block_result(cid, tokio::fs::read(path).await)
	}

	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), skip(block), fields(cid = ?block.cid(), path = ?to_cid_path(&self.path, block.cid(), "")))]
	async fn set(&self, block: Block) -> Result<Cid, StorageError> {
		let path = to_cid_path(&self.path, block.cid(), "");

		// exists?
		match tokio::fs::metadata(&path).await {
			// run some validations and skip re-write
			Ok(m) => {
				if !m.is_file() {
					return Err(StorageError::Internal(anyhow!("Unexpected file type: {:?}", path)));
				}
				if m.len() != block.data().len() as u64 {
					return Err(StorageError::Internal(anyhow!(
						"Unexpected file size: {} != {}: {:?}",
						m.len(),
						block.data().len(),
						path,
					)));
				}
				return Ok(block.into_inner().0);
			},
			// continue with write
			Err(e) if e.kind() == ErrorKind::NotFound => {},
			// forward other errors (permission, ...)
			Err(e) => return Err(StorageError::Internal(e.into())),
		}

		// write
		fs_write_atomic(&path, block.data(), true)
			.await
			.map_err(|e| StorageError::Internal(e.into()))?;

		// result
		Ok(block.into_inner().0)
	}

	async fn remove(&self, cid: &Cid) -> Result<(), StorageError> {
		let path = to_cid_path(&self.path, cid, "");
		into_storage_result(cid, tokio::fs::remove_file(path).await)
	}

	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError> {
		let path = to_cid_path(&self.path, cid, "");
		into_storage_result(cid, tokio::fs::metadata(&path).await.map(|v| BlockStat { size: v.size() }))
	}

	fn max_block_size(&self) -> usize {
		self.max_block_size
	}
}
#[async_trait]
impl ExtendedBlockStorage for FsStorage {
	async fn set_extended(&self, block: ExtendedBlock) -> Result<Cid, StorageError> {
		self.set(block.block).await
	}

	async fn exists(&self, cid: &Cid) -> Result<bool, StorageError> {
		let path = to_cid_path(&self.path, cid, "");
		into_storage_result(cid, tokio::fs::try_exists(&path).await)
	}

	async fn clear(&self) -> Result<(), StorageError> {
		if self.allow_clear {
			match tokio::fs::remove_dir_all(&self.path).await {
				Ok(_) => Ok(()),
				Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
				Err(err) => Err(StorageError::Internal(err.into())),
			}
		} else {
			Err(StorageError::InvalidArgument(anyhow!("Clear disallowed: {}", self.path.to_string_lossy())))
		}
	}
}
impl CloneWithBlockStorageSettings for FsStorage {
	fn clone_with_settings(&self, _settings: BlockStorageCloneSettings) -> Self {
		self.clone()
	}
}
#[async_trait]
impl BlockStorageContentMapping for FsStorage {}

/// Convert io result to storage result.
fn into_storage_result<T>(cid: &Cid, result: std::io::Result<T>) -> Result<T, StorageError> {
	match result {
		Ok(data) => Ok(data),
		Err(e) if e.kind() == ErrorKind::NotFound => Err(StorageError::NotFound(*cid, e.into())),
		Err(e) => Err(StorageError::Internal(anyhow::Error::from(e).context(format!("Reading CID: {}", cid)))),
	}
}

/// Convert io result to block result.
fn into_block_result(cid: &Cid, result: std::io::Result<Vec<u8>>) -> Result<Block, StorageError> {
	into_storage_result(cid, result).map(|data| Block::new_unchecked(*cid, data))
}

fn to_cid_path(path: &Path, cid: &Cid, prefix: &str) -> PathBuf {
	let mut folder = cid
		.hash()
		.digest()
		.iter()
		// .next_chunk::<2>()
		.map(|chunk| format!("{:02x}", chunk))
		.take(2)
		.fold(path.to_owned(), |mut result, next| {
			result.push(next);
			result
		});
	folder.push(format!("{}{}", prefix, cid));
	folder
}

#[cfg(test)]
mod tests {
	use super::to_cid_path;
	use crate::FsStorage;
	use cid::Cid;
	use co_primitives::{Block, BlockStorage, BlockStorageExt};
	use co_test::TmpDir;
	use std::{path::PathBuf, str::FromStr};

	#[test]
	fn test_to_cid_path() {
		let cid = Cid::from_str("bafyr4igf663hpuvdpvque42uxmkbacg5ubd4cgageulmwmqo33g2tpod7e").unwrap();
		assert_eq!(
			to_cid_path(&PathBuf::from("/test"), &cid, ""),
			PathBuf::from("/test/c5/f7/bafyr4igf663hpuvdpvque42uxmkbacg5ubd4cgageulmwmqo33g2tpod7e"),
		);
		assert_eq!(
			to_cid_path(&PathBuf::from("/test"), &cid, "."),
			PathBuf::from("/test/c5/f7/.bafyr4igf663hpuvdpvque42uxmkbacg5ubd4cgageulmwmqo33g2tpod7e"),
		);
	}

	#[tokio::test]
	async fn smoke() {
		let tmp = TmpDir::new("co");
		let storage = FsStorage::new(tmp.path().to_owned());
		let cid = storage.set_serialized(&1).await.unwrap();
		let value: i32 = storage.get_deserialized(&cid).await.unwrap();
		assert_eq!(value, 1);
	}

	/// Regression coverage for a same-CID write race: many concurrent writers targeting the
	/// exact same content (and therefore the exact same `.{cid}` temp path) must all succeed,
	/// and the final file must contain the complete block.
	#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
	async fn concurrent_set_same_cid() {
		const BLOCK_SIZE: usize = 4 * 1024 * 1024;

		for round in 0u8..6 {
			let tmp = TmpDir::new("co");
			let storage = FsStorage::new(tmp.path().to_owned());
			let block = Block::new_data(0x55u64, vec![round; BLOCK_SIZE]);
			let cid = *block.cid();

			let mut tasks = Vec::with_capacity(64);
			for _ in 0..64 {
				let storage = storage.clone();
				let block = block.clone();
				tasks.push(tokio::spawn(async move { storage.set(block).await }));
			}

			for task in tasks {
				let result = task.await.expect("concurrent_set_same_cid: task panicked");
				assert!(
					result.is_ok(),
					"round {round}: concurrent set of identical block returned an error: {:?}",
					result.err()
				);
			}

			match storage.stat(&cid).await {
				Ok(stat) => {
					assert_eq!(stat.size, BLOCK_SIZE as u64, "round {round}: unexpected final block size on disk");
				},
				Err(err) => panic!("round {round}: stat after concurrent set failed: {err:?}"),
			}
		}
	}

	/// Regression coverage for an unflushed-write race: `set` must not rename a temp file into
	/// place until all of its bytes are actually durable, even for writes spanning more than one
	/// of tokio::fs::File's internal write chunks (2 MiB by default).
	#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
	async fn set_publishes_complete_bytes() {
		const BLOCK_SIZE: usize = 4 * 1024 * 1024;

		let tmp = TmpDir::new("co");
		let storage = FsStorage::new(tmp.path().to_owned());

		for round in 0u8..20 {
			let mut data = vec![0u8; BLOCK_SIZE];
			data[0] = round;
			let expected = data.len();
			let block = Block::new_data(0x55u64, data);
			let cid = *block.cid();

			storage
				.set(block)
				.await
				.unwrap_or_else(|err| panic!("round {round}: set failed: {err:?}"));

			// read back synchronously and immediately (no tokio, no yielding) so a rename that
			// overtook an in-flight background write is observed as a short read.
			let path = to_cid_path(tmp.path(), &cid, "");
			let actual = std::fs::read(&path)
				.unwrap_or_else(|err| panic!("round {round}: reading {path:?} failed: {err:?}"))
				.len();
			assert_eq!(actual, expected, "round {round}: read {actual} bytes from {path:?}, expected {expected}");
		}
	}
}
