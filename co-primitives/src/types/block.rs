use cid::Cid;
use multihash_codetable::{Code, MultihashDigest};
use std::{
	hash::{Hash, Hasher},
	marker::PhantomData,
};

pub trait StoreParams: std::fmt::Debug + Clone + Send + Sync + Unpin + 'static {
	const MAX_BLOCK_SIZE: usize;
}
#[derive(Debug, Clone)]
pub struct DefaultParams {}
impl StoreParams for DefaultParams {
	const MAX_BLOCK_SIZE: usize = 1_048_576;
}

/// Block
#[derive(Clone)]
pub struct Block<S> {
	_s: PhantomData<S>,
	cid: Cid,
	data: Vec<u8>,
}
impl<S: StoreParams> Block<S> {
	/// Creates a new block. Returns an error if the hash doesn't match
	/// the data.
	pub fn new(cid: Cid, data: Vec<u8>) -> Result<Self, BlockError> {
		verify_cid::<multihash_codetable::Code, 64>(&cid, &data)?;
		Ok(Self::new_unchecked(cid, data))
	}

	/// Creates a new block without verifying the cid.
	pub fn new_unchecked(cid: Cid, data: Vec<u8>) -> Self {
		Self { _s: Default::default(), cid, data }
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
		Cid::new_v1(codec.into(), digest.digest(&data))
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

	/// Block with specific store params.
	pub fn with_store_params<P>(self) -> Result<Block<P>, BlockError>
	where
		P: StoreParams,
	{
		if self.data.len() > P::MAX_BLOCK_SIZE {
			return Err(BlockError::SizeOverflow(self.data.len(), P::MAX_BLOCK_SIZE));
		}
		Ok(Block::new_unchecked(self.cid, self.data))
	}
}
impl<S> PartialEq for Block<S> {
	fn eq(&self, other: &Self) -> bool {
		self.cid == other.cid
	}
}
impl<S> Eq for Block<S> {}
impl<S> PartialOrd for Block<S> {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		self.cid.partial_cmp(&other.cid)
	}
}
impl<S> Ord for Block<S> {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.cid.cmp(&other.cid)
	}
}
impl<S> Hash for Block<S> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		Hash::hash(&self, state);
	}
}
impl<S> std::fmt::Debug for Block<S> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Block")
			.field("cid", &self.cid)
			.field("data", &self.data)
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
