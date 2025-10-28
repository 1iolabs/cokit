use crate::{
	library::wait_response::request_response, services::application::NetworkBlockGetAction, Action, ApplicationMessage,
};
use async_trait::async_trait;
use cid::Cid;
use co_actor::ActorHandle;
use co_primitives::{Block, BlockStorageSettings, CloneWithBlockStorageSettings, CoId, MappedCid};
use co_storage::{
	BlockStat, BlockStorage, BlockStorageContentMapping, ExtendedBlock, ExtendedBlockStorage, StorageError,
};
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct ReducerBlockStorage<S> {
	parent_co: CoId,
	co: CoId,
	next: S,
	handle: ActorHandle<ApplicationMessage>,
	settings: BlockStorageSettings,
}
impl<S> ReducerBlockStorage<S>
where
	S: BlockStorage + BlockStorageContentMapping + Send + Sync + Clone + 'static,
{
	pub fn new(
		parent_co: CoId,
		co: CoId,
		next: S,
		handle: ActorHandle<ApplicationMessage>,
		settings: BlockStorageSettings,
	) -> Self {
		Self { parent_co, co, next, handle, settings }
	}

	async fn get_handle(&self, cid: Cid) -> Result<(), StorageError> {
		let action = NetworkBlockGetAction { parent_co: self.parent_co.clone(), co: self.co.clone(), cid };

		// request
		request_response(self.handle.clone(), Action::NetworkBlockGet(action.clone()), move |item| match item {
			Action::NetworkBlockGetComplete(complete, result) if complete == &action => Some(result.clone()),
			_ => None,
		})
		.await??;

		// done
		Ok(())
	}
}
#[async_trait]
impl<S> BlockStorage for ReducerBlockStorage<S>
where
	S: BlockStorage + BlockStorageContentMapping + Send + Sync + Clone + 'static,
{
	type StoreParams = S::StoreParams;

	/// Returns a block from storage.
	async fn get(&self, cid: &Cid) -> Result<Block<Self::StoreParams>, StorageError> {
		match self.next.get(cid).await {
			Ok(block) => Ok(block),
			Err(StorageError::NotFound(_, _)) if !self.settings.disallow_networking => {
				self.get_handle(*cid).await?;
				self.next.get(cid).await
			},
			Err(e) => Err(e),
		}
	}

	/// Inserts a block into storage.
	/// Returns the CID of the block (gurranted to be the same as the supplied).
	async fn set(&self, block: Block<Self::StoreParams>) -> Result<Cid, StorageError> {
		self.next.set(block).await
	}

	/// Remove a block.
	async fn remove(&self, cid: &Cid) -> Result<(), StorageError> {
		self.next.remove(cid).await
	}

	/// Stat a block.
	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError> {
		match self.next.stat(cid).await {
			Err(StorageError::NotFound(_, _)) if !self.settings.disallow_networking => {
				self.get_handle(*cid).await?;
				self.next.stat(cid).await
			},
			result => result,
		}
	}
}
#[async_trait]
impl<S> ExtendedBlockStorage for ReducerBlockStorage<S>
where
	S: BlockStorage + ExtendedBlockStorage + BlockStorageContentMapping + Send + Sync + Clone + 'static,
{
	async fn set_extended(&self, block: ExtendedBlock<Self::StoreParams>) -> Result<Cid, StorageError> {
		self.next.set_extended(block).await
	}

	async fn clear(&self) -> Result<(), StorageError> {
		self.next.clear().await
	}

	async fn exists(&self, cid: &Cid) -> Result<bool, StorageError> {
		self.next.exists(cid).await
	}
}
impl<S> CloneWithBlockStorageSettings for ReducerBlockStorage<S>
where
	S: CloneWithBlockStorageSettings,
{
	fn clone_with_settings(&self, settings: BlockStorageSettings) -> Self {
		Self {
			parent_co: self.parent_co.clone(),
			co: self.co.clone(),
			next: self.next.clone_with_settings(settings.clone()),
			settings,
			handle: self.handle.clone(),
		}
	}
}
#[async_trait]
impl<S> BlockStorageContentMapping for ReducerBlockStorage<S>
where
	S: BlockStorage + BlockStorageContentMapping + Send + Sync + Clone + 'static,
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

	async fn insert_mappings(&self, mappings: BTreeSet<MappedCid>) {
		self.next.insert_mappings(mappings).await
	}
}
