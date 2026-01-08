use crate::{CoCid, CoError};
use co_primitives::StoreParams;
use co_sdk::DefaultParams;
use std::sync::Arc;

#[cfg_attr(feature = "frb", flutter_rust_bridge::frb(opaque))]
#[derive(Clone)]
pub struct BlockStorage {
	storage: Arc<dyn co_sdk::BlockStorage<StoreParams = DefaultParams> + 'static>,
}

impl BlockStorage {
	#[cfg_attr(feature = "frb", flutter_rust_bridge::frb(ignore))]
	pub fn new(storage: impl co_sdk::BlockStorage<StoreParams = DefaultParams> + 'static) -> Self {
		Self { storage: Arc::new(storage) }
	}

	pub async fn get(&self, cid: &CoCid) -> Result<Block, CoError> {
		Ok(self.storage.get(&cid.cid()?).await.map_err(CoError::new)?.into())
	}

	pub async fn set(&self, block: Block) -> Result<CoCid, CoError> {
		Ok(self.storage.set(block.try_into()?).await.map_err(CoError::new)?.into())
	}
}

#[derive(Debug, Clone)]
pub struct Block {
	pub cid: CoCid,
	pub data: Vec<u8>,
}
impl Block {
	/// Creates a new block. Returns an error if the hash doesn't match
	/// the data.
	pub fn new(cid: CoCid, data: Vec<u8>) -> Result<Self, CoError> {
		let (cid, data) = co_sdk::Block::<co_sdk::DefaultParams>::new(cid.cid()?, data)
			.map_err(CoError::new)?
			.into_inner();
		Ok(Self { cid: cid.into(), data })
	}

	/// Creates a new block without verifying the cid.
	pub fn new_unchecked(cid: CoCid, data: Vec<u8>) -> Self {
		Self { cid, data }
	}

	/// Create a new block by calculating the [`Cid`] from data using the default hasher.
	/// Note: The default hasher may changes without notice.
	pub fn new_data(codec: u64, data: Vec<u8>) -> Self {
		Self::new_unchecked(co_sdk::Block::<DefaultParams>::cid_data(codec, &data).into(), data)
	}
}
impl<P: StoreParams> From<co_sdk::Block<P>> for Block {
	fn from(value: co_sdk::Block<P>) -> Self {
		let (cid, data) = value.into_inner();
		Self { cid: cid.into(), data }
	}
}
impl<P: StoreParams> TryFrom<Block> for co_sdk::Block<P> {
	type Error = CoError;

	fn try_from(value: Block) -> Result<Self, Self::Error> {
		Self::new(value.cid.cid()?, value.data).map_err(CoError::new)
	}
}
