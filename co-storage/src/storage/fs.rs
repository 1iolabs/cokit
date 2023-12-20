use crate::{Storage, StorageError};
use anyhow::anyhow;
use libipld::{Block, Cid, DefaultParams};
use std::{io::ErrorKind, path::PathBuf};

/// Filesystem storage.
///
/// Creates one file per CID.
/// To ensure directories arend getting too many entries extra folders are created for the furst two bytes of the CID
/// digest.
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
	fn get(&self, cid: &Cid) -> Result<Block<DefaultParams>, StorageError> {
		let path = to_cid_path(&self.path, cid);
		match std::fs::read(path) {
			Ok(data) => Ok(Block::new_unchecked(cid.clone(), data)),
			Err(e) if e.kind() == ErrorKind::NotFound => Err(StorageError::NotFound(cid.clone())),
			Err(e) => Err(StorageError::Internal(e.into())),
		}
	}

	fn set(&mut self, block: Block<DefaultParams>) -> Result<(), StorageError> {
		let path = to_cid_path(&self.path, block.cid());

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
				return Ok(())
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
		std::fs::write(path, block.data()).map_err(|e| StorageError::Internal(e.into()))?;

		// result
		Ok(())
	}
}

fn to_cid_path(path: &PathBuf, cid: &Cid) -> PathBuf {
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
	folder.push(cid.to_string());
	folder
}

#[cfg(test)]
mod tests {
	use super::to_cid_path;
	use libipld::Cid;
	use std::{path::PathBuf, str::FromStr};

	#[test]
	fn test_to_cid_path() {
		let cid = Cid::from_str("bafyr4igf663hpuvdpvque42uxmkbacg5ubd4cgageulmwmqo33g2tpod7e").unwrap();
		assert_eq!(
			to_cid_path(&PathBuf::from("/test"), &cid),
			PathBuf::from("/test/c5/f7/bafyr4igf663hpuvdpvque42uxmkbacg5ubd4cgageulmwmqo33g2tpod7e"),
		);
	}
}
