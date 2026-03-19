// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{AnyBlockStorage, Block, BlockStorage, KnownMultiCodec, MultiCodec, StorageError};
use cid::Cid;
use futures::{AsyncRead, AsyncReadExt};
use rust_unixfs::file::{adder::FileAdder, visit::IdleFileVisit};

/// Read unixfs file into buffer.
///
/// See: https://github.com/dariusc93/rust-ipfs/blob/libp2p-next/unixfs/examples/cat.rs
pub async fn unixfs_cat_buffer(storage: &impl AnyBlockStorage, cid: &Cid) -> Result<Vec<u8>, StorageError> {
	let mut result = Vec::new();

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
			.get(MultiCodec::with_codec(KnownMultiCodec::DagPb, cid)?)
			.await?
			.into_inner()
			.1,
	);

	// First step of the walk can give content or continued visitation but not both.
	let (content, _, _metadata, mut step) = IdleFileVisit::default()
		.start(&buf)
		.map_err(|e| StorageError::Internal(e.into()))?;
	result.extend_from_slice(content);

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
		result.extend_from_slice(content);

		// Using a while loop combined with `let Some(visit) = step` allows for easy walking.
		step = next_step;
	}

	// result
	Ok(result)
}

/// Add stream as unixfs file to storage.
/// The last CID in the result is the root.
///
/// See: https://github.com/dariusc93/rust-ipfs/blob/libp2p-next/unixfs/examples/add.rs
pub async fn unixfs_add<I>(storage: &impl AnyBlockStorage, stream: &mut I) -> Result<Vec<Cid>, StorageError>
where
	I: AsyncRead + Unpin,
{
	let mut result = Vec::new();
	let mut adder = FileAdder::default();
	let mut buf = vec![0u8; 16384];
	loop {
		// read
		let bytes = stream.read(&mut buf).await.map_err(|e| StorageError::Internal(e.into()))?;
		if bytes == 0 {
			let blocks = adder.finish();
			add_blocks(storage, blocks, &mut result).await?;
			break;
		}

		// process
		let mut total = 0;
		while total < bytes {
			let (blocks, consumed) = adder.push(&buf[total..bytes]);
			add_blocks(storage, blocks, &mut result).await?;
			total += consumed;
		}
	}
	Ok(result)
}

/// Encode buffer into blocks.
/// The last block in the result is the root.
pub fn unixfs_encode_buffer(buf: &[u8]) -> Vec<Block> {
	let mut result = Vec::new();
	let mut adder = FileAdder::default();

	// push
	let mut total = 0;
	while total < buf.len() {
		let (blocks, consumed) = adder.push(&buf[total..]);
		for (cid, data) in blocks {
			result.push(Block::new_unchecked(cid, data));
		}
		total += consumed;
	}

	// finish
	for (cid, data) in adder.finish() {
		result.push(Block::new_unchecked(cid, data));
	}

	// result
	result
}

/// Add blocks to storage and add its CID's to `cids`.
async fn add_blocks<S>(
	storage: &S,
	blocks: impl Iterator<Item = (Cid, Vec<u8>)>,
	cids: &mut Vec<Cid>,
) -> Result<(), StorageError>
where
	S: BlockStorage + Send,
{
	for (cid, data) in blocks {
		let block = Block::new_unchecked(cid, data);
		let cid = storage.set(block).await?;
		cids.push(cid);
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use crate::{unixfs_add, unixfs_cat_buffer, TestStorage};
	use cid::Cid;
	use futures::io::Cursor;
	use std::str::FromStr;

	/// Test 1MiB of data.
	/// Note that the leaf blocks are the same because they contain the same data.
	#[tokio::test]
	async fn test_unixfs_add() {
		let storage = TestStorage::default();
		let mut stream = Cursor::new("hello world test".repeat(64).repeat(1024).as_bytes().to_vec()); // 1024KiB
		let cids = unixfs_add(&storage, &mut stream).await.unwrap();
		// println!("cids: {:?}", cids);
		assert_eq!(5, cids.len());
		assert_eq!(cids[0], Cid::from_str("QmPEvxGmvxzfMews81gF5NMvFNeFAdNmhtwzGPhkHhoyqy").unwrap());
		assert_eq!(cids[1], Cid::from_str("QmPEvxGmvxzfMews81gF5NMvFNeFAdNmhtwzGPhkHhoyqy").unwrap());
		assert_eq!(cids[2], Cid::from_str("QmPEvxGmvxzfMews81gF5NMvFNeFAdNmhtwzGPhkHhoyqy").unwrap());
		assert_eq!(cids[3], Cid::from_str("QmPEvxGmvxzfMews81gF5NMvFNeFAdNmhtwzGPhkHhoyqy").unwrap());
		assert_eq!(cids[4], Cid::from_str("QmVRRmYKvn8m3jQT8VHX1BCgrQLFvzsB26aKwLCyFRvYSv").unwrap());
	}

	#[tokio::test]
	async fn test_unixfs_add_empty() {
		let storage = TestStorage::default();
		let mut stream = Cursor::new([]);
		let cids = unixfs_add(&storage, &mut stream).await.unwrap();
		assert_eq!(1, cids.len());
		assert_eq!(cids[0], Cid::from_str("QmbFMke1KXqnYyBBWxB74N4c5SBnJMVAiMNRcGu6x1AwQH").unwrap());
	}

	#[tokio::test]
	async fn test_unixfs_cat_buffer() {
		let storage = TestStorage::default();
		let data = "hello world test".repeat(64).repeat(1024); // 1024KiB
		let mut stream = Cursor::new(data.as_bytes().to_vec());
		let cids = unixfs_add(&storage, &mut stream).await.unwrap();
		let buffer = unixfs_cat_buffer(&storage, cids.last().unwrap()).await.unwrap();
		assert_eq!(data.as_bytes().to_vec(), buffer);
	}
}
