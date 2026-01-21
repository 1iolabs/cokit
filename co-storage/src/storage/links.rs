use crate::{BlockStorageContentMapping, ExtendedBlock, ExtendedBlockStorage};
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{
	Block, BlockLinks, BlockStat, BlockStorage, BlockStorageCloneSettings, BlockStorageStoreParams,
	CloneWithBlockStorageSettings, MappedCid, StorageError,
};
use std::collections::BTreeSet;

/// A [`BlockStorage`] which verfies all links exist when create a new block.
#[derive(Debug, Default, Clone)]
pub struct LinksBlockStorage<S> {
	links: Option<BlockLinks>,
	next: S,
}
impl<S> LinksBlockStorage<S> {
	pub fn new(next: S, links: Option<BlockLinks>) -> Self {
		Self { next, links }
	}
}
#[async_trait]
impl<S> BlockStorage for LinksBlockStorage<S>
where
	S: BlockStorage + 'static,
{
	async fn get(&self, cid: &Cid) -> Result<Block, StorageError> {
		self.next.get(cid).await
	}

	async fn set(&self, block: Block) -> Result<Cid, StorageError> {
		// verify all links exist
		if let Some(block_links) = &self.links {
			if block_links.has_links(block.cid()) {
				let links = block_links.links(&block)?;
				for link in links {
					match self.next.get(&link).await {
						Ok(_) => {},
						Err(err) => {
							let err = StorageError::InvalidArgument(anyhow::Error::from(err).context(format!(
								"Create block failed: {} reference failed: {}",
								block.cid(),
								&link
							)));
							tracing::error!(?err, "create-block-failed");
							return Err(err);
						},
					}
				}
			}
		}

		// next
		self.next.set(block).await
	}

	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError> {
		self.next.stat(cid).await
	}

	async fn remove(&self, cid: &Cid) -> Result<(), StorageError> {
		self.next.remove(cid).await
	}

	fn max_block_size(&self) -> usize {
		self.next.max_block_size()
	}
}
#[async_trait]
impl<S> ExtendedBlockStorage for LinksBlockStorage<S>
where
	S: ExtendedBlockStorage + 'static,
{
	async fn set_extended(&self, block: ExtendedBlock) -> Result<Cid, StorageError> {
		self.next.set_extended(block).await
	}

	async fn exists(&self, cid: &Cid) -> Result<bool, StorageError> {
		self.next.exists(cid).await
	}

	async fn clear(&self) -> Result<(), StorageError> {
		self.next.clear().await
	}
}
#[async_trait]
impl<S> BlockStorageContentMapping for LinksBlockStorage<S>
where
	S: BlockStorage + BlockStorageContentMapping + 'static,
{
	async fn is_content_mapped(&self) -> bool {
		self.next.is_content_mapped().await
	}

	async fn to_plain(&self, mapped: &Cid) -> Option<Cid> {
		self.next.to_plain(mapped).await
	}

	async fn to_mapped(&self, plain: &Cid) -> Option<Cid> {
		self.next.to_mapped(plain).await
	}

	async fn insert_mappings(&self, mappings: BTreeSet<MappedCid>) {
		self.next.insert_mappings(mappings).await
	}
}
#[async_trait]
impl<S> CloneWithBlockStorageSettings for LinksBlockStorage<S>
where
	S: BlockStorage + CloneWithBlockStorageSettings + 'static,
{
	fn clone_with_settings(&self, settings: BlockStorageCloneSettings) -> Self {
		Self::new(self.next.clone_with_settings(settings), self.links.clone())
	}
}
impl<S> BlockStorageStoreParams for LinksBlockStorage<S>
where
	S: BlockStorageStoreParams,
{
	type StoreParams = S::StoreParams;
}
