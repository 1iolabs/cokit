use crate::{BlockStorageContentMapping, Storage};
use anyhow::anyhow;
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{
	Block, BlockStat, BlockStorage, BlockStorageSettings, CloneWithBlockStorageSettings, DefaultParams, StorageError,
	StoreParams,
};
use std::{
	fs::OpenOptions,
	io::{ErrorKind, Write},
	os::unix::fs::MetadataExt,
	path::{Path, PathBuf},
	thread::sleep,
	time::Duration,
};
use tokio::io::AsyncWriteExt;

/// Filesystem storage.
///
/// Creates one file per CID.
/// To ensure directories arend getting too many entries extra folders are created for the furst two bytes of the CID
/// digest.
#[derive(Debug, Clone)]
pub struct FsStorage {
	path: PathBuf,
}
impl FsStorage {
	pub fn new(path: PathBuf) -> Self {
		Self { path }
	}

	pub fn create(&self) -> std::io::Result<()> {
		std::fs::create_dir_all(&self.path)
	}
}
impl Storage for FsStorage {
	type StoreParams = DefaultParams;

	fn get(&self, cid: &Cid) -> Result<Block<DefaultParams>, StorageError> {
		let path = to_cid_path(&self.path, cid, "");
		into_block_result(cid, std::fs::read(path))
	}

	fn set(&mut self, block: Block<DefaultParams>) -> Result<Cid, StorageError> {
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

		// create parents
		if let Some(parent) = path.parent() {
			std::fs::create_dir_all(parent).map_err(|e| StorageError::Internal(e.into()))?;
		}

		// write
		let tmp_path = to_cid_path(&self.path, block.cid(), ".");
		let mut retry = 3;
		loop {
			retry -= 1;

			// write exclusive to tmp_path
			fn write(write_path: &Path, write_data: &[u8]) -> std::io::Result<()> {
				Ok(OpenOptions::new()
					.write(true)
					.create_new(true)
					.open(write_path)?
					.write_all(write_data)?)
			}
			match write(&tmp_path, block.data()) {
				Ok(_) => {},
				Err(err) if retry > 0 && err.kind() == ErrorKind::AlreadyExists => {
					// wait for other process to complete
					sleep(Duration::from_millis(10));

					// now exists?
					if std::fs::metadata(&path).is_ok() {
						break;
					}

					// retry
					continue;
				},
				Err(err) => {
					// make sure the tmp file is gone
					std::fs::remove_file(&tmp_path).ok();

					// forward error
					Err(err).map_err(|e| StorageError::Internal(e.into()))?;
				},
			}

			// move to path
			std::fs::rename(tmp_path, path).map_err(|e| StorageError::Internal(e.into()))?;
			break;
		}

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
	type StoreParams = DefaultParams;

	async fn get(&self, cid: &Cid) -> Result<Block<Self::StoreParams>, StorageError> {
		let path = to_cid_path(&self.path, cid, "");
		into_block_result(cid, tokio::fs::read(path).await)
	}

	#[tracing::instrument(level = tracing::Level::TRACE, err, skip(block), fields(cid = ?block.cid(), path = ?to_cid_path(&self.path, block.cid(), "")))]
	async fn set(&self, block: Block<Self::StoreParams>) -> Result<Cid, StorageError> {
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

		// create parents
		if let Some(parent) = path.parent() {
			tokio::fs::create_dir_all(parent)
				.await
				.map_err(|e| StorageError::Internal(e.into()))?;
		}

		// write
		let tmp_path = to_cid_path(&self.path, block.cid(), ".");
		let mut retry = 3;
		loop {
			retry -= 1;

			// write exclusive to tmp_path
			async fn write(write_path: &Path, write_data: &[u8]) -> std::io::Result<()> {
				Ok(tokio::fs::OpenOptions::new()
					.write(true)
					.create_new(true)
					.open(write_path)
					.await?
					.write_all(write_data)
					.await?)
			}
			match write(&tmp_path, block.data()).await {
				Ok(_) => {},
				Err(err) if retry > 0 && err.kind() == ErrorKind::AlreadyExists => {
					// wait for other process to complete
					tokio::time::sleep(Duration::from_millis(10)).await;

					// now exists?
					if tokio::fs::metadata(&path).await.is_ok() {
						break;
					}

					// retry
					continue;
				},
				Err(err) => {
					// make sure the tmp file is gone
					tokio::fs::remove_file(&tmp_path).await.ok();

					// forward error
					Err(err).map_err(|e| StorageError::Internal(e.into()))?;
				},
			}

			// move to path
			tokio::fs::rename(tmp_path, path)
				.await
				.map_err(|e| StorageError::Internal(e.into()))?;
			break;
		}

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
}
impl CloneWithBlockStorageSettings for FsStorage {
	fn clone_with_settings(&self, _settings: BlockStorageSettings) -> Self {
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
fn into_block_result<P: StoreParams>(cid: &Cid, result: std::io::Result<Vec<u8>>) -> Result<Block<P>, StorageError> {
	into_storage_result(cid, result).map(|data| Block::new_unchecked(*cid, data))
}

fn to_cid_path(path: &PathBuf, cid: &Cid, prefix: &str) -> PathBuf {
	let mut folder = cid
		.hash()
		.digest()
		.iter()
		// .next_chunk::<2>()
		.map(|chunk| format!("{:02x}", chunk))
		.take(2)
		.fold(path.clone(), |mut result, next| {
			result.push(next);
			result
		});
	folder.push(format!("{}{}", prefix, cid));
	folder
}

#[cfg(test)]
mod tests {
	use super::to_cid_path;
	use crate::{FsStorage, TmpDir};
	use cid::Cid;
	use co_primitives::BlockStorageExt;
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
}
