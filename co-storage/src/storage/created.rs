use async_trait::async_trait;
use cid::Cid;
use co_primitives::{Block, BlockStat, BlockStorage, StorageError};
use futures::lock::Mutex;
use std::{collections::HashSet, mem::swap, sync::Arc};

/// Store all [`Cid`] of blocks that have been newly created.
/// Additionally set calls for blocks which already exists in `next` will be ignored.
#[derive(Debug, Clone)]
pub struct CreatedBlockStorage<S> {
	next: S,
	created: Arc<Mutex<HashSet<Cid>>>,
}
impl<S> CreatedBlockStorage<S> {
	pub fn new(next: S) -> Self {
		Self { next, created: Default::default() }
	}

	/// Drain all created items and return them as iterator.
	pub async fn drain(&self) -> impl Iterator<Item = Cid> + use<S> {
		let mut created = self.created.lock().await;
		let mut result = HashSet::new();
		swap(&mut result, &mut created);
		result.into_iter()
	}
}
#[async_trait]
impl<S> BlockStorage for CreatedBlockStorage<S>
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
		self.created.lock().await.insert(result);

		// result
		Ok(result)
	}

	async fn remove(&self, cid: &Cid) -> Result<(), StorageError> {
		// remove
		let result = self.next.remove(cid).await?;

		// record
		self.created.lock().await.remove(cid);

		// result
		Ok(result)
	}

	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError> {
		Ok(self.next.stat(cid).await?)
	}
}
