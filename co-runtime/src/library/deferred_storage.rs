// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::RuntimeContext;
use anyhow::anyhow;
use cid::Cid;
use co_primitives::{AnyBlockStorage, Block, BlockLinks, DefaultParams, KnownMultiCodec, StorageError};
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
		// use all direct links from input (which is state and action for ReducerInput)
		let input_block = Block::new_data(KnownMultiCodec::DagCbor, runtime_context.input.clone());
		let mut stack: VecDeque<(Cid, u8)> = links.links(&input_block)?.map(|cid| (cid, 1)).collect();

		// fetch blocks and follow one level of links
		let mut blocks = Vec::new();
		while let Some((cid, depth)) = stack.pop_front() {
			let block = storage.get(&cid).await?;

			if depth > 0 && links.has_links(block.cid()) {
				for link in links.links(&block)? {
					stack.push_back((link, depth - 1));
				}
			}

			blocks.push(block);
		}

		// store
		let mut context = self.context.lock().unwrap();
		for block in blocks {
			context.blocks.insert(*block.cid(), block);
		}
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
