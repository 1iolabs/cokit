use crate::{BlockStat, BlockStorage, BlockStorageContentMapping, ExtendedBlock, ExtendedBlockStorage, StorageError};
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{Block, BlockStorageSettings, CloneWithBlockStorageSettings, StoreParams};
use std::{collections::BTreeMap, marker::PhantomData};

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
		self.next.set(convert_block_store_params(block, self.checked)?).await
	}

	async fn remove(&self, cid: &Cid) -> Result<(), StorageError> {
		self.next.remove(cid).await
	}

	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError> {
		self.next.stat(cid).await
	}
}
#[async_trait]
impl<S, P> ExtendedBlockStorage for StoreParamsBlockStorage<S, P>
where
	S: ExtendedBlockStorage + Send + Sync + Clone,
	P: StoreParams,
{
	async fn set_extended(&self, block: ExtendedBlock<Self::StoreParams>) -> Result<Cid, StorageError> {
		let next_block =
			ExtendedBlock { block: convert_block_store_params(block.block, self.checked)?, options: block.options };
		self.next.set_extended(next_block).await
	}
}
impl<S, P> CloneWithBlockStorageSettings for StoreParamsBlockStorage<S, P>
where
	S: BlockStorage + CloneWithBlockStorageSettings,
	P: Clone,
{
	fn clone_with_settings(&self, settings: BlockStorageSettings) -> Self {
		Self { next: self.next.clone_with_settings(settings), checked: self.checked, _p: Default::default() }
	}
}
#[async_trait]
impl<S, P> BlockStorageContentMapping for StoreParamsBlockStorage<S, P>
where
	S: BlockStorage + CloneWithBlockStorageSettings + BlockStorageContentMapping + 'static,
	P: StoreParams,
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

	async fn insert_mappings(&self, mappings: BTreeMap<Cid, Cid>) {
		self.next.insert_mappings(mappings).await
	}
}

fn convert_block_store_params<I, O>(block: Block<I>, checked: bool) -> Result<Block<O>, StorageError>
where
	I: StoreParams,
	O: StoreParams,
{
	let (cid, data) = block.into_inner();
	let next_block: Block<O> = match checked {
		true => Block::new(cid, data).map_err(|e| StorageError::InvalidArgument(e.into()))?,
		false => Block::new_unchecked(cid, data),
	};
	Ok(next_block)
}
