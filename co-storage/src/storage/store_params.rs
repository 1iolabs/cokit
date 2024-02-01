use crate::{BlockStat, BlockStorage, StorageError};
use async_trait::async_trait;
use libipld::{store::StoreParams, Block, Cid};
use std::marker::PhantomData;

/// This storage implementation converts block storeparams.
/// If an conversation is not possible `StorageError::InvalidArgument` is retuned.
#[derive(Debug, Clone)]
pub struct StoreParamsBlockStorage<S, P>
where
	S: Clone,
{
	_p: PhantomData<P>,
	next: S,
	checked: bool,
}
impl<S, P> StoreParamsBlockStorage<S, P>
where
	S: Clone,
{
	pub fn new(next: S, checked: bool) -> Self {
		Self { _p: Default::default(), next, checked }
	}
}
#[async_trait]
impl<S, P> BlockStorage for StoreParamsBlockStorage<S, P>
where
	S: BlockStorage + Send + Sync + Clone,
	P: StoreParams,
{
	type StoreParams = P;

	async fn get(&self, cid: &Cid) -> Result<Block<Self::StoreParams>, StorageError> {
		let (cid, data) = self.next.get(cid).await?.into_inner();
		match self.checked {
			true => Ok(Block::new(cid, data).map_err(|e| StorageError::InvalidArgument(e.into()))?),
			false => Ok(Block::new_unchecked(cid, data)),
		}
	}

	async fn set(&self, block: Block<Self::StoreParams>) -> Result<Cid, StorageError> {
		let (cid, data) = block.into_inner();
		let next_block: Block<S::StoreParams> = match self.checked {
			true => Block::new(cid, data).map_err(|e| StorageError::InvalidArgument(e.into()))?,
			false => Block::new_unchecked(cid, data),
		};
		self.next.set(next_block).await
	}

	async fn remove(&self, cid: &Cid) -> Result<(), StorageError> {
		self.next.remove(cid).await
	}

	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError> {
		self.next.stat(cid).await
	}
}
