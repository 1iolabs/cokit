use crate::{Block, BlockStorage, BlockStorageSettings, CloneWithBlockStorageSettings, DefaultParams, StorageError};
use anyhow::anyhow;
use async_trait::async_trait;
use cid::Cid;
use std::{
	collections::BTreeMap,
	sync::{Arc, Mutex},
};

#[derive(Debug, Default, Clone)]
pub struct TestStorage {
	items: Arc<Mutex<BTreeMap<Cid, Block<DefaultParams>>>>,
}
#[async_trait]
impl BlockStorage for TestStorage {
	type StoreParams = DefaultParams;

	async fn get(&self, cid: &Cid) -> Result<Block<Self::StoreParams>, StorageError> {
		self.items
			.lock()
			.unwrap()
			.get(cid)
			.ok_or_else(|| StorageError::NotFound(*cid, anyhow!("No record")))
			.cloned()
	}
	async fn set(&self, block: Block<Self::StoreParams>) -> Result<Cid, StorageError> {
		let cid = *block.cid();
		self.items.lock().unwrap().insert(cid, block);
		Ok(cid)
	}
	async fn remove(&self, cid: &Cid) -> Result<(), StorageError> {
		self.items.lock().unwrap().remove(cid);
		Ok(())
	}
}
impl CloneWithBlockStorageSettings for TestStorage {
	fn clone_with_settings(&self, _settings: BlockStorageSettings) -> Self {
		self.clone()
	}
}
