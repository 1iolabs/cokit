use crate::{BlockStorageContentMapping, ExtendedBlock, ExtendedBlockStorage};
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{Block, BlockStat, BlockStorage, CloneWithBlockStorageSettings, MappedCid, StorageError};
use std::{
	collections::{BTreeSet, HashSet},
	mem::swap,
	sync::{Arc, Mutex},
};

/// Store all [`Cid`] of blocks that have been newly created or removed.
/// Additionally set calls for blocks which already exists in `next` will be ignored.
#[derive(Debug, Clone)]
pub struct ChangeBlockStorage<S> {
	next: S,
	changes: Arc<Mutex<HashSet<BlockStorageChange>>>,
	record: bool,
}
impl<S> ChangeBlockStorage<S> {
	pub fn new(next: S) -> Self {
		Self { next, changes: Default::default(), record: true }
	}

	pub fn set_record(&mut self, record: bool) {
		self.record = record;
	}

	/// Drain all changes and return them as iterator.
	pub async fn drain(&self) -> impl Iterator<Item = BlockStorageChange> + use<S> {
		let mut created = self.changes.lock().unwrap();
		let mut result = HashSet::new();
		swap(&mut result, &mut created);
		result.into_iter()
	}
}
#[async_trait]
impl<S> BlockStorage for ChangeBlockStorage<S>
where
	S: BlockStorage + 'static,
{
	type StoreParams = S::StoreParams;

	async fn get(&self, cid: &Cid) -> Result<Block<Self::StoreParams>, StorageError> {
		Ok(self.next.get(cid).await?)
	}

	async fn set(&self, block: Block<Self::StoreParams>) -> Result<Cid, StorageError> {
		// already exists?
		if let Ok(_) = self.next.stat(block.cid()).await {
			return Ok(*block.cid());
		}

		// create
		let result = self.next.set(block).await?;

		// record
		if self.record {
			let mut changes = self.changes.lock().unwrap();
			changes.remove(&BlockStorageChange::Remove(result));
			changes.insert(BlockStorageChange::Set(result));
		}

		// result
		Ok(result)
	}

	async fn remove(&self, cid: &Cid) -> Result<(), StorageError> {
		// remove
		let result = self.next.remove(cid).await?;

		// record (ignore when it just has been added)
		if self.record {
			let mut changes = self.changes.lock().unwrap();
			if !changes.remove(&BlockStorageChange::Set(*cid)) {
				changes.insert(BlockStorageChange::Remove(*cid));
			}
		}

		// result
		Ok(result)
	}

	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError> {
		Ok(self.next.stat(cid).await?)
	}
}
#[async_trait]
impl<S> ExtendedBlockStorage for ChangeBlockStorage<S>
where
	S: ExtendedBlockStorage + 'static,
{
	async fn set_extended(&self, block: ExtendedBlock<Self::StoreParams>) -> Result<Cid, StorageError> {
		self.next.set_extended(block).await
	}

	async fn exists(&self, cid: &Cid) -> Result<bool, StorageError> {
		self.next.exists(cid).await
	}

	async fn clear(&self) -> Result<(), StorageError> {
		self.next.clear().await
	}
}
impl<S> CloneWithBlockStorageSettings for ChangeBlockStorage<S>
where
	S: BlockStorage + CloneWithBlockStorageSettings + 'static,
{
	fn clone_with_settings(&self, settings: co_primitives::BlockStorageSettings) -> Self {
		Self { next: self.next.clone_with_settings(settings), changes: self.changes.clone(), record: self.record }
	}
}
#[async_trait]
impl<S> BlockStorageContentMapping for ChangeBlockStorage<S>
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

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum BlockStorageChange {
	Set(Cid),
	Remove(Cid),
}
