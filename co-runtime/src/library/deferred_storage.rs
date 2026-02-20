use anyhow::anyhow;
use cid::Cid;
use co_primitives::{AnyBlockStorage, Block, DefaultParams, StorageError};
use co_storage::Storage;
use std::{
	collections::HashMap,
	sync::{Arc, Mutex},
};

#[derive(Debug, Clone, Default)]
pub struct DeferredStorage {
	context: Arc<Mutex<DeferredStorageContext>>,
}
impl DeferredStorage {
	/// Process all deferred storage operations.
	///
	/// # Args
	/// - `flush_set` - if `true`, then flush all blocks that has been set to storage
	pub async fn process(&mut self, storage: &impl AnyBlockStorage, flush_set: bool) -> Result<bool, StorageError> {
		let mut context = self.context.lock().unwrap();
		let mut retry = false;
		while !context.ops.is_empty() {
			let op = context.ops.remove(0);
			match op {
				DeferredOps::Get(cid) => {
					let block = storage.get(&cid).await?;
					context.blocks.insert(*block.cid(), block);
				},
				DeferredOps::Set(cid) => {
					if flush_set {
						if let Some(block) = context.blocks.remove(&cid) {
							storage.set(block).await?;
						}
					} else if let Some(block) = context.blocks.get(&cid) {
						storage.set(block.clone()).await?;
					}
				},
				DeferredOps::Remove(cid) => {
					storage.remove(&cid).await?;
				},
			}
			retry = true;
		}
		Ok(retry)
	}
}
impl Storage for DeferredStorage {
	type StoreParams = DefaultParams;

	fn get(&self, cid: &Cid) -> Result<Block, StorageError> {
		let mut context = self.context.lock().unwrap();
		match context.blocks.get(cid).cloned() {
			Some(block) => Ok(block),
			None => {
				context.ops.push(DeferredOps::Get(*cid));
				Err(StorageError::Internal(anyhow!("Block pending")))
			},
		}
	}

	fn set(&mut self, block: Block) -> Result<Cid, StorageError> {
		let mut context = self.context.lock().unwrap();
		let cid = *block.cid();
		if !context.blocks.contains_key(&cid) {
			context.ops.push(DeferredOps::Set(cid));
			context.blocks.insert(cid, block);
		}
		Ok(cid)
	}

	fn remove(&mut self, cid: &Cid) -> Result<(), StorageError> {
		let mut context = self.context.lock().unwrap();
		context.ops.push(DeferredOps::Remove(*cid));
		context.blocks.remove(cid);
		Ok(())
	}
}

#[derive(Debug)]
enum DeferredOps {
	Get(Cid),
	Set(Cid),
	Remove(Cid),
}

#[derive(Debug, Default)]
struct DeferredStorageContext {
	blocks: HashMap<Cid, Block>,
	ops: Vec<DeferredOps>,
}
impl DeferredStorageContext {}
