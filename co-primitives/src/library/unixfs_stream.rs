// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{BlockStorage, KnownMultiCodec, MultiCodec, StorageError};
use cid::Cid;
use core::ops::Range;
use futures::Stream;
use rust_unixfs::file::visit::IdleFileVisit;

/// Read unixfs file as chunked stream.
///
/// See: https://github.com/dariusc93/rust-ipfs/blob/libp2p-next/unixfs/examples/cat.rs
pub fn unixfs_stream<S>(
	storage: S,
	cid: Cid,
	range: Option<Range<u64>>,
) -> impl Stream<Item = Result<Vec<u8>, StorageError>>
where
	S: BlockStorage + Send,
{
	async_stream::try_stream! {
		let mut visit = IdleFileVisit::default();
		if let Some(range) = range {
			visit = visit.with_target_range(range);
		}

		// The blockstore specific way of reading the block. Here we assume go-ipfs 0.5 default flatfs
		// configuration, which puts the files at sharded directories and names the blocks as base32
		// upper and a suffix of "data".
		//
		// For the ipfs-unixfs it is important that the raw block data lives long enough that the
		// possible content gets to be processed, at minimum one step of the walk as shown in this
		// example.
		let mut buf = Vec::new();
		buf.append(
			&mut storage
				.get(MultiCodec::with_codec(KnownMultiCodec::DagPb, &cid)?)
				.await?
				.into_inner()
				.1,
		);

		// First step of the walk can give content or continued visitation but not both.
		let (content, _, _metadata, mut step) = visit
			.start(&buf)
			.map_err(|e| StorageError::Internal(e.into()))?;
		yield content.to_vec();

		// Following steps repeat the same pattern:
		while let Some(visit) = step {
			// Read the next link. The `pending_links()` gives the next link and an iterator over the
			// following links. The iterator lists the known links in the order of traversal, with the
			// exception of possible new links appearing before the older.
			let (first, _) = visit.pending_links();

			buf.clear();
			buf.append(&mut storage.get(first).await?.into_inner().1);

			// Similar to first step, except we no longer get the file metadata. It is still accessible
			// from the `visit` via `AsRef<ipfs_unixfs::file::Metadata>` but likely only needed in
			// the first step.
			let (content, next_step) = visit
				.continue_walk(&buf, &mut None)
				.map_err(|e| StorageError::Internal(e.into()))?;
			yield content.to_vec();

			// Using a while loop combined with `let Some(visit) = step` allows for easy walking.
			step = next_step;
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::{unixfs_add, unixfs_stream, TestStorage};
	use futures::{io::Cursor, TryStreamExt};

	#[tokio::test]
	async fn test_unixfs_stream() {
		let storage = TestStorage::default();
		let data = "hello world test".repeat(64).repeat(1024); // 1024KiB
		let mut stream = Cursor::new(data.as_bytes().to_vec());
		let cids = unixfs_add(&storage, &mut stream).await.unwrap();
		let buffer = unixfs_stream(storage, *cids.last().unwrap(), None)
			.try_collect::<Vec<_>>()
			.await
			.unwrap()
			.concat();
		assert_eq!(buffer, data.as_bytes().to_vec());
	}

	#[tokio::test]
	async fn test_unixfs_stream_range() {
		let storage = TestStorage::default();
		let data = "hello world test".repeat(64).repeat(1024); // 1024KiB
		let mut stream = Cursor::new(data.as_bytes().to_vec());
		let cids = unixfs_add(&storage, &mut stream).await.unwrap();
		let range = 512 * 1024..786 * 1024;
		let buffer = unixfs_stream(storage, *cids.last().unwrap(), Some(range.clone()))
			.try_collect::<Vec<_>>()
			.await
			.unwrap()
			.concat();
		let data_bytes = data.as_bytes();
		assert_eq!(&buffer[..], &data_bytes[512 * 1024..786 * 1024]);
	}

	#[tokio::test]
	async fn test_unixfs_stream_range_unaligned() {
		let storage = TestStorage::default();
		let data = "hello world test".repeat(64).repeat(1024); // 1024KiB
		let mut stream = Cursor::new(data.as_bytes().to_vec());
		let cids = unixfs_add(&storage, &mut stream).await.unwrap();
		let range = 10000..10016;
		let buffer = unixfs_stream(storage, *cids.last().unwrap(), Some(range.clone()))
			.try_collect::<Vec<_>>()
			.await
			.unwrap()
			.concat();
		assert_eq!(&buffer[..], "hello world test".as_bytes());
	}
}
