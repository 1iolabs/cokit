use crate::RuntimeContext;
use anyhow::anyhow;
use cid::Cid;
use co_primitives::{AnyBlockStorage, Block, BlockLinks, DefaultParams, StorageError};
use co_storage::Storage;
use std::{
	collections::{HashMap, VecDeque},
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
		let mut retry = false;
		loop {
			let op = {
				let mut context = self.context.lock().unwrap();
				if context.ops.is_empty() {
					break;
				}
				context.ops.remove(0)
			};
			match op {
				DeferredOps::Get(cid) => {
					let block = storage.get(&cid).await?;
					let mut context = self.context.lock().unwrap();
					context.blocks.insert(*block.cid(), block);
					retry = true;
				},
				DeferredOps::Set(cid) => {
					let block = {
						let mut context = self.context.lock().unwrap();
						if flush_set {
							context.blocks.remove(&cid)
						} else {
							context.blocks.get(&cid).cloned()
						}
					};
					if let Some(block) = block {
						storage.set(block).await?;
					}
				},
				DeferredOps::Remove(cid) => {
					storage.remove(&cid).await?;
				},
			}
		}
		Ok(retry)
	}

	/// Warm the storage cache so we dont need to retry immediately.
	pub async fn warm(
		&mut self,
		storage: &impl AnyBlockStorage,
		links: &BlockLinks,
		runtime_context: &RuntimeContext,
	) -> Result<(), anyhow::Error> {
		// stack
		let mut stack = VecDeque::with_capacity(10);
		if let Some(cid) = runtime_context.state {
			stack.push_back((cid, 0));
		}
		stack.push_back((runtime_context.event, 1));

		// process
		let mut blocks = Vec::new();
		while let Some((cid, depth)) = stack.pop_front() {
			let block = storage.get(&cid).await?;

			// links
			if depth > 0 && links.has_links(block.cid()) {
				for link in links.links(&block)? {
					stack.push_back((link, depth - 1));
				}
			}

			// add
			blocks.push(block);
		}

		// store
		let mut context = self.context.lock().unwrap();
		for block in blocks {
			context.blocks.insert(*block.cid(), block);
		}

		// result
		Ok(())
	}
}
impl Storage for DeferredStorage {
	type StoreParams = DefaultParams;

	fn get(&self, cid: &Cid) -> Result<Block, StorageError> {
		let mut context = self.context.lock().unwrap();
		match context.blocks.get(cid).cloned() {
			Some(block) => Ok(block),
			None => {
				tracing::trace!(?cid, "deferred-block-pending");
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
