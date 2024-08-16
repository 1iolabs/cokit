use crate::{BlockStat, BlockStorage, BlockStorageContentMapping, StorageError};
use async_trait::async_trait;
use co_primitives::MultiCodec;
use libipld::{Block, Cid};
use std::collections::BTreeSet;

/// Mappes certain CID codecs to mapped CIDs using BlockStorageContentMapping.
pub struct MappedBlockStorage<S, M> {
	storage: S,
	mapping: M,
	codecs: BTreeSet<MultiCodec>,
}
impl<S, M> MappedBlockStorage<S, M>
where
	S: BlockStorage + Send + Sync + 'static,
	M: BlockStorageContentMapping + Send + Sync + 'static,
{
	pub fn new(storage: S, mapping: M, codecs: BTreeSet<MultiCodec>) -> Self {
		Self { mapping, codecs, storage }
	}

	pub async fn to_mapped(&self, cid: &Cid) -> Cid {
		let codec = cid.into();
		if self.codecs.contains(&codec) {
			match self.mapping.to_mapped(cid).await {
				Some(mapped) => mapped,
				None => *cid,
			}
		} else {
			*cid
		}
	}
}
#[async_trait]
impl<S, M> BlockStorage for MappedBlockStorage<S, M>
where
	S: BlockStorage + Send + Sync + 'static,
	M: BlockStorageContentMapping + Send + Sync + 'static,
{
	type StoreParams = S::StoreParams;

	/// Returns a block from storage.
	async fn get(&self, cid: &Cid) -> Result<Block<Self::StoreParams>, StorageError> {
		Ok(self.storage.get(&self.to_mapped(cid).await).await?)
	}

	/// Inserts a block into storage.
	/// Returns the CID of the block (gurranted to be the same as the supplied).
	async fn set(&self, block: Block<Self::StoreParams>) -> Result<Cid, StorageError> {
		Ok(self.storage.set(block).await?)
	}

	/// Remove a block.
	async fn remove(&self, cid: &Cid) -> Result<(), StorageError> {
		Ok(self.storage.remove(&self.to_mapped(cid).await).await?)
	}

	/// Stat a block.
	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError> {
		Ok(self.storage.stat(&self.to_mapped(cid).await).await?)
	}
}
