use anyhow::Result;
use co_storage::{BlockStorage, StorageError};
use libipld::{Block, Cid, DefaultParams};
use libp2p_bitswap::BitswapStore;

/// Wrap BlockStorage as BitswapStore
pub struct BitswapBlockStorage<S> {
	storage: S,
}
impl<S> BitswapBlockStorage<S> {
	pub fn new(storage: S) -> Self {
		Self { storage }
	}
}

/// Handle (external) peer requests.
#[async_trait::async_trait]
impl<S> BitswapStore for BitswapBlockStorage<S>
where
	S: BlockStorage<StoreParams = DefaultParams> + Send + Sync + 'static,
{
	type Params = S::StoreParams;

	async fn contains(&mut self, cid: &Cid) -> Result<bool> {
		match self.storage.stat(cid).await {
			Ok(_) => Ok(true),
			Err(StorageError::NotFound(_, _)) => Ok(false),
			Err(e) => Err(e.into()),
		}
	}

	async fn get(&mut self, cid: &Cid) -> Result<Option<Vec<u8>>> {
		match self.storage.get(cid).await {
			Ok(block) => Ok(Some(block.into_inner().1)),
			Err(StorageError::NotFound(_, _)) => Ok(None),
			Err(e) => Err(e.into()),
		}
	}

	async fn insert(&mut self, block: &Block<Self::Params>) -> Result<()> {
		self.storage.set(block.clone()).await?;
		Ok(())
	}

	async fn missing_blocks(&mut self, cid: &Cid) -> Result<Vec<Cid>> {
		let mut stack = vec![*cid];
		let mut missing = vec![];
		while let Some(cid) = stack.pop() {
			if let Some(data) = self.get(&cid).await? {
				let block = Block::<Self::Params>::new_unchecked(cid, data);
				block.references(&mut stack)?;
			} else {
				missing.push(cid);
			}
		}
		Ok(missing)
	}
}
