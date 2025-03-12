use async_trait::async_trait;
use cid::Cid;
use co_primitives::{Block, BlockStat, BlockStorage, StorageError};
use futures::lock::Mutex;
use std::{collections::HashSet, mem::swap, sync::Arc};

/// Store all [`Cid`] of blocks that have been newly created or removed.
/// Additionally set calls for blocks which already exists in `next` will be ignored.
#[derive(Debug, Clone)]
pub struct ChangeBlockStorage<S> {
	next: S,
	changes: Arc<Mutex<HashSet<BlockStorageChange>>>,
}
impl<S> ChangeBlockStorage<S> {
	pub fn new(next: S) -> Self {
		Self { next, changes: Default::default() }
	}

	/// Drain all changes and return them as iterator.
	pub async fn drain(&self) -> impl Iterator<Item = BlockStorageChange> + use<S> {
		let mut created = self.changes.lock().await;
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
		match self.next.stat(block.cid()).await {
			Ok(_) => return Ok(*block.cid()),
			_ => {},
		}

		// create
		let result = self.next.set(block).await?;

		// record
		let mut changes = self.changes.lock().await;
		changes.remove(&BlockStorageChange::Remove(result));
		changes.insert(BlockStorageChange::Set(result));

		// result
		Ok(result)
	}

	async fn remove(&self, cid: &Cid) -> Result<(), StorageError> {
		// remove
		let result = self.next.remove(cid).await?;

		// record (ignore when it just has been added)
		let mut changes = self.changes.lock().await;
		if !changes.remove(&BlockStorageChange::Set(*cid)) {
			changes.insert(BlockStorageChange::Remove(*cid));
		}

		// result
		Ok(result)
	}

	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError> {
		Ok(self.next.stat(cid).await?)
	}
}

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum BlockStorageChange {
	Set(Cid),
	Remove(Cid),
}
