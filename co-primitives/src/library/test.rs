use crate::{
	types::block_storage::BlockStorageStoreParams, Block, BlockStorage, BlockStorageCloneSettings,
	CloneWithBlockStorageSettings, DefaultParams, StorageError, StoreParams,
};
use anyhow::anyhow;
use async_trait::async_trait;
use cid::Cid;
use std::{
	collections::BTreeMap,
	sync::{Arc, Mutex},
};

#[derive(Debug, Default, Clone)]
pub struct TestStorage {
	items: Arc<Mutex<BTreeMap<Cid, Block>>>,
}
#[async_trait]
impl BlockStorage for TestStorage {
	async fn get(&self, cid: &Cid) -> Result<Block, StorageError> {
		self.items
			.lock()
			.unwrap()
			.get(cid)
			.ok_or_else(|| StorageError::NotFound(*cid, anyhow!("No record")))
			.cloned()
	}
	async fn set(&self, block: Block) -> Result<Cid, StorageError> {
		let cid = *block.cid();
		self.items
			.lock()
			.unwrap()
			.insert(cid, block.with_store_params::<<Self as BlockStorageStoreParams>::StoreParams>()?);
		Ok(cid)
	}
	async fn remove(&self, cid: &Cid) -> Result<(), StorageError> {
		self.items.lock().unwrap().remove(cid);
		Ok(())
	}
	fn max_block_size(&self) -> usize {
		<Self as BlockStorageStoreParams>::StoreParams::MAX_BLOCK_SIZE
	}
}
impl BlockStorageStoreParams for TestStorage {
	type StoreParams = DefaultParams;
}
impl CloneWithBlockStorageSettings for TestStorage {
	fn clone_with_settings(&self, _settings: BlockStorageCloneSettings) -> Self {
		self.clone()
	}
}
