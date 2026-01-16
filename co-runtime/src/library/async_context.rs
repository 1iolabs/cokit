use crate::RuntimeContext;
use async_trait::async_trait;
use cid::Cid;
use co_api::{async_api, Block, BlockStorage, DefaultParams, StorageError, StoreParams};
use co_storage::StoreParamsBlockStorage;
use std::sync::Arc;

pub struct AsyncContext {
	storage: AsyncBlockStorage,
	context: RuntimeContext,
}
impl AsyncContext {
	pub fn new<S>(storage: S, context: RuntimeContext, checked: bool) -> Self
	where
		S: BlockStorage + Clone + 'static,
	{
		Self { storage: AsyncBlockStorage::new(storage, checked), context }
	}

	pub fn context(self) -> RuntimeContext {
		self.context
	}
}
impl async_api::Context<AsyncBlockStorage> for AsyncContext {
	fn storage(&self) -> &AsyncBlockStorage {
		&self.storage
	}

	fn payload(&self) -> Vec<u8> {
		self.context.payload.clone()
	}

	fn event(&self) -> Cid {
		self.context.event
	}

	fn state(&self) -> Option<Cid> {
		self.context.state
	}

	fn set_state(&mut self, cid: Cid) {
		self.context.state = Some(cid);
	}

	fn write_diagnostic(&mut self, cid: Cid) {
		self.context.diagnostics.push(cid.into());
	}
}

#[derive(Clone)]
pub struct AsyncBlockStorage(Arc<dyn BlockStorage + 'static>);
impl AsyncBlockStorage {
	fn new<S>(storage: S, checked: bool) -> Self
	where
		S: BlockStorage + Clone + 'static,
	{
		// we do not want dynamic lbokc limits in cores as this breaks determinism
		//  we force all blocks created on cores to be max. 1MiB
		Self(Arc::new(StoreParamsBlockStorage::new(storage, checked, DefaultParams::MAX_BLOCK_SIZE)))
	}
}
#[async_trait]
impl BlockStorage for AsyncBlockStorage {
	/// Returns a block from storage.
	async fn get(&self, cid: &Cid) -> Result<Block, StorageError> {
		Ok(self.0.get(cid).await?)
	}

	/// Inserts a block into storage.
	async fn set(&self, block: Block) -> Result<Cid, StorageError> {
		Ok(self.0.set(block).await?)
	}

	/// Remove a block.
	async fn remove(&self, cid: &Cid) -> Result<(), StorageError> {
		Ok(self.0.remove(cid).await?)
	}

	/// Maximum accepted block size.
	fn max_block_size(&self) -> usize {
		self.0.max_block_size()
	}
}
