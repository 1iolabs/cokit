use crate::{BlockStat, BlockStorage, BlockStorageContentMapping, StorageError};
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{Block, CloneWithBlockStorageSettings, MultiCodec};
use std::collections::BTreeSet;

/// Mappes certain CID codecs to mapped CIDs using BlockStorageContentMapping.
#[derive(Debug, Clone)]
pub struct MappedBlockStorage<S> {
	storage: S,
	codecs: BTreeSet<MultiCodec>,
}
impl<S> MappedBlockStorage<S>
where
	S: BlockStorage + BlockStorageContentMapping + Send + Sync + 'static,
{
	pub fn new(storage: S, codecs: BTreeSet<MultiCodec>) -> Self {
		Self { codecs, storage }
	}

	pub async fn to_mapped(&self, cid: &Cid) -> Cid {
		let codec = cid.into();
		if self.codecs.contains(&codec) {
			match self.storage.to_mapped(cid).await {
				Some(mapped) => mapped,
				None => *cid,
			}
		} else {
			*cid
		}
	}
}
#[async_trait]
impl<S> BlockStorage for MappedBlockStorage<S>
where
	S: BlockStorage + BlockStorageContentMapping + Send + Sync + 'static,
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
impl<S> CloneWithBlockStorageSettings for MappedBlockStorage<S>
where
	S: CloneWithBlockStorageSettings,
{
	fn clone_with_settings(&self, settings: co_primitives::BlockStorageSettings) -> Self {
		MappedBlockStorage { storage: self.storage.clone_with_settings(settings), codecs: self.codecs.clone() }
	}
}
#[async_trait]
impl<S> BlockStorageContentMapping for MappedBlockStorage<S>
where
	S: BlockStorage + BlockStorageContentMapping + 'static,
{
	async fn is_content_mapped(&self) -> bool {
		self.storage.is_content_mapped().await
	}

	async fn to_plain(&self, mapped: &Cid) -> Option<Cid> {
		self.storage.to_plain(mapped).await
	}

	async fn to_mapped(&self, plain: &Cid) -> Option<Cid> {
		self.storage.to_mapped(plain).await
	}
}
