// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{MultiCodec, StoreParams};
use cid::Cid;
use multihash_codetable::{Code, MultihashDigest};
use std::hash::{Hash, Hasher};

/// Block
#[derive(Clone)]
pub struct Block {
	cid: Cid,
	data: Vec<u8>,
}
impl Block {
	/// Creates a new block. Returns an error if the hash doesn't match
	/// the data.
	pub fn new(cid: Cid, data: Vec<u8>) -> Result<Self, BlockError> {
		verify_cid::<multihash_codetable::Code, 64>(&cid, &data)?;
		Ok(Self::new_unchecked(cid, data))
	}

	/// Creates a new block without verifying the cid.
	pub fn new_unchecked(cid: Cid, data: Vec<u8>) -> Self {
		Self { cid, data }
	}

	/// Create a new block by calculating the [`Cid`] from data using the default hasher.
	/// Note: The default hasher may changes without notice.
	pub fn new_data(codec: impl Into<u64>, data: Vec<u8>) -> Self {
		Self::new_unchecked(Self::cid_data(codec, &data), data)
	}

	/// Create a new block by calculating the [`Cid`] from data.
	pub fn new_data_digest(digest: impl MultihashDigest<64>, codec: impl Into<u64>, data: Vec<u8>) -> Self {
		Self::new_unchecked(Self::cid_data_digest(digest, codec, &data), data)
	}

	pub fn cid_data(codec: impl Into<u64>, data: &[u8]) -> Cid {
		Self::cid_data_digest(Code::Blake3_256, codec, data)
	}

	pub fn cid_data_digest(digest: impl MultihashDigest<64>, codec: impl Into<u64>, data: &[u8]) -> Cid {
		Cid::new_v1(codec.into(), digest.digest(data))
	}

	/// Returns the cid.
	pub fn cid(&self) -> &Cid {
		&self.cid
	}

	/// Returns the payload.
	pub fn data(&self) -> &[u8] {
		&self.data
	}

	/// Returns the inner cid and data.
	pub fn into_inner(self) -> (Cid, Vec<u8>) {
		(self.cid, self.data)
	}

	/// Block with verified hash.
	pub fn with_verify(self) -> Result<Block, BlockError> {
		verify_cid::<multihash_codetable::Code, 64>(&self.cid, &self.data)?;
		Ok(self)
	}

	/// Block with specific store params.
	pub fn with_store_params<P: StoreParams>(self) -> Result<Block, BlockError> {
		self.with_block_max_size(P::MAX_BLOCK_SIZE)
	}

	/// Block with specific store max size.
	pub fn with_block_max_size(self, max_block_size: usize) -> Result<Block, BlockError> {
		if self.data.len() > max_block_size {
			return Err(BlockError::SizeOverflow(self.data.len(), max_block_size));
		}
		Ok(self)
	}
}
impl PartialEq for Block {
	fn eq(&self, other: &Self) -> bool {
		self.cid == other.cid
	}
}
impl Eq for Block {}
impl PartialOrd for Block {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}
impl Ord for Block {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.cid.cmp(&other.cid)
	}
}
impl Hash for Block {
	fn hash<H: Hasher>(&self, state: &mut H) {
		Hash::hash(&self, state);
	}
}
impl std::fmt::Debug for Block {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let hex = self.data.iter().map(|c| format!("{:02X}", c)).collect::<String>();
		let codec = MultiCodec::from(&self.cid);
		f.debug_struct("Block")
			.field("cid", &self.cid)
			.field("codec", &codec)
			.field("size", &self.data.len())
			.field("data", &hex)
			.finish()
	}
}

#[derive(Debug, thiserror::Error)]
pub enum BlockError {
	#[error("Unsupported codec {0:?}.")]
	UnsupportedCodec(u64),

	#[error("Unsupported multihash {0:?}.")]
	UnsupportedMultihash(u64),

	#[error("Hash of data does not match the CID.")]
	InvalidMultihash(Vec<u8>, Cid),
	// #[error("Serialize failed: {0:?}: {1}")]
	// Serialize(MultiCodec, #[source] anyhow::Error),
	#[error("Max block size overflow ({0} > {1})")]
	SizeOverflow(usize, usize),
}

fn verify_cid<M: MultihashDigest<S>, const S: usize>(cid: &Cid, payload: &[u8]) -> Result<(), BlockError> {
	let mh = M::try_from(cid.hash().code())
		.map_err(|_| BlockError::UnsupportedMultihash(cid.hash().code()))?
		.digest(payload);
	if mh.digest() != cid.hash().digest() {
		return Err(BlockError::InvalidMultihash(mh.to_bytes(), *cid));
	}
	Ok(())
}
