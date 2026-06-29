// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use std::{
	io::{self, ErrorKind, Write},
	path::Path,
};
use tempfile::NamedTempFile;
use tokio::fs;

pub async fn fs_write(path: impl AsRef<Path>, contents: impl AsRef<[u8]>, create_dir_all: bool) -> io::Result<()> {
	match fs::write(path.as_ref(), contents.as_ref()).await {
		Err(e) if create_dir_all && e.kind() == ErrorKind::NotFound => {
			// create parent dir
			fs::create_dir_all(path.as_ref().parent().ok_or::<io::Error>(ErrorKind::NotFound.into())?).await?;

			// retry write
			fs::write(path, contents).await
		},
		i => i,
	}
}

/// Atomically write `contents` to `path` via a uniquely-named temp file + `rename`.
/// Safe for concurrent readers (they see the complete old or new file)
/// and concurrent writers (each uses its own temp).
///
/// # Notes
/// - No `fsync`: durability is intentionally not provided.
pub async fn fs_write_atomic(
	path: impl AsRef<Path>,
	contents: impl AsRef<[u8]>,
	create_dir_all: bool,
) -> io::Result<()> {
	let path = path.as_ref().to_path_buf();
	let contents = contents.as_ref().to_vec();

	// spawn as blocking
	//  note: internally [`tokio::fs::write`] is using spawn_blocking too
	tokio::task::spawn_blocking(move || -> io::Result<()> {
		// the temp must live in the same directory as the target (same filesystem => atomic rename)
		let dir = match path.parent() {
			Some(parent) if !parent.as_os_str().is_empty() => parent,
			_ => Path::new("."),
		};
		if create_dir_all {
			std::fs::create_dir_all(dir)?;
		}

		// unique temp; auto-removed on drop unless we persist it
		let mut tmp = NamedTempFile::new_in(dir)?;
		tmp.write_all(&contents)?;

		// atomically replace the target; on failure the temp is returned and dropped (cleaned up)
		tmp.persist(&path).map_err(|e| e.error)?;
		Ok(())
	})
	.await
	.map_err(io::Error::other)?
}

#[cfg(test)]
mod tests {
	use super::fs_write_atomic;
	use co_test::TmpDir;

	#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
	async fn test_fs_write_atomic_concurrent_same_path() {
		let tmp = TmpDir::new("co");
		let path = tmp.path().join("file.bin");

		// many concurrent writers to the SAME path - each must succeed and write a complete file
		let mut handles = vec![];
		for i in 0..64u32 {
			let path = path.clone();
			handles.push(tokio::spawn(async move {
				let data = vec![i as u8; 4096];
				fs_write_atomic(&path, &data, true).await
			}));
		}
		for h in handles {
			// with a fixed temp name, concurrent writers race on the shared temp and some fail
			// (e.g. rename ENOENT after another writer renamed it away); with unique temps all succeed
			h.await.unwrap().unwrap();
		}

		// the final file is exactly one writer's complete content (uniform bytes), never a torn mix
		let data = tokio::fs::read(&path).await.unwrap();
		assert_eq!(data.len(), 4096);
		let first = data[0];
		assert!(data.iter().all(|&b| b == first), "content must come from a single writer, not torn");

		// free cleanup: only the target remains, no stray temp files
		let mut names = vec![];
		let mut entries = tokio::fs::read_dir(tmp.path()).await.unwrap();
		while let Some(e) = entries.next_entry().await.unwrap() {
			names.push(e.file_name().to_string_lossy().into_owned());
		}
		assert_eq!(names, vec!["file.bin".to_string()], "only the target should remain, got {names:?}");
	}

	#[tokio::test]
	async fn test_fs_write_atomic_creates_and_overwrites() {
		let tmp = TmpDir::new("co");
		let path = tmp.path().join("nested").join("file.bin");

		// create (with parent dir)
		fs_write_atomic(&path, b"v1", true).await.unwrap();
		assert_eq!(tokio::fs::read(&path).await.unwrap(), b"v1");

		// overwrite with different-length content
		fs_write_atomic(&path, b"version-two", true).await.unwrap();
		assert_eq!(tokio::fs::read(&path).await.unwrap(), b"version-two");

		// no temp file left behind
		assert!(!path.with_file_name("file.bin.tmp").exists());
	}
}
