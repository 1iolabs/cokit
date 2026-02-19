// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::CoContext;
use anyhow::anyhow;
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{Block, BlockStorageCloneSettings, CloneWithBlockStorageSettings, StoreParams};
use co_sdk::{
	Application, BlockStat, BlockStorage, BlockStorageContentMapping, CoId, CoStorage, DefaultParams, StorageError,
};
use dioxus::{
	hooks::{use_callback, use_context, use_reactive},
	prelude::use_hook,
};
use futures::Future;
use tokio::sync::{mpsc, oneshot};

pub fn use_co_storage(co: &String) -> CoBlockStorage {
	let mut co_id = use_reactive(co, CoId::from);
	let context: CoContext = use_context();
	let storage = use_callback(move |_| {
		let (tx, mut rx) = mpsc::unbounded_channel::<Command>();
		context.execute_future_parallel(|application| async move {
			while let Some(command) = rx.recv().await {
				handle_command(&application, command).await;
			}
		});
		CoBlockStorage { co: co_id(), tx, settings: None }
	});
	use_hook(|| storage(()))
}

#[derive(Debug, Clone)]
pub struct CoBlockStorage {
	co: CoId,
	settings: Option<BlockStorageCloneSettings>,
	tx: mpsc::UnboundedSender<Command>,
}
#[async_trait]
impl BlockStorage for CoBlockStorage {
	/// Returns a block from storage.
	async fn get(&self, cid: &Cid) -> Result<Block, StorageError> {
		let (result_tx, result_rx) = oneshot::channel();
		self.tx
			.send(Command::Get(self.co.clone(), *cid, self.settings.clone(), result_tx))
			.map_err(|err| StorageError::Internal(err.into()))?;
		result_rx.await.map_err(|err| StorageError::Internal(err.into()))?
	}

	/// Inserts a block into storage.
	/// Returns the CID of the block (gurranted to be the same as the supplied).
	async fn set(&self, block: Block) -> Result<Cid, StorageError> {
		let (result_tx, result_rx) = oneshot::channel();
		self.tx
			.send(Command::Set(self.co.clone(), block, self.settings.clone(), result_tx))
			.map_err(|err| StorageError::Internal(err.into()))?;
		result_rx.await.map_err(|err| StorageError::Internal(err.into()))?
	}

	/// Remove a block.
	async fn remove(&self, cid: &Cid) -> Result<(), StorageError> {
		let (result_tx, result_rx) = oneshot::channel();
		self.tx
			.send(Command::Remove(self.co.clone(), *cid, self.settings.clone(), result_tx))
			.map_err(|err| StorageError::Internal(err.into()))?;
		result_rx.await.map_err(|err| StorageError::Internal(err.into()))?
	}

	/// Stat a block.
	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError> {
		let (result_tx, result_rx) = oneshot::channel();
		self.tx
			.send(Command::Stat(self.co.clone(), *cid, self.settings.clone(), result_tx))
			.map_err(|err| StorageError::Internal(err.into()))?;
		result_rx.await.map_err(|err| StorageError::Internal(err.into()))?
	}

	fn max_block_size(&self) -> usize {
		DefaultParams::MAX_BLOCK_SIZE
	}
}
impl CloneWithBlockStorageSettings for CoBlockStorage {
	fn clone_with_settings(&self, settings: BlockStorageCloneSettings) -> Self {
		CoBlockStorage { co: self.co.clone(), settings: Some(settings), tx: self.tx.clone() }
	}
}
impl BlockStorageContentMapping for CoBlockStorage {}

enum Command {
	Get(CoId, Cid, Option<BlockStorageCloneSettings>, oneshot::Sender<Result<Block, StorageError>>),
	Set(CoId, Block, Option<BlockStorageCloneSettings>, oneshot::Sender<Result<Cid, StorageError>>),
	Remove(CoId, Cid, Option<BlockStorageCloneSettings>, oneshot::Sender<Result<(), StorageError>>),
	Stat(CoId, Cid, Option<BlockStorageCloneSettings>, oneshot::Sender<Result<BlockStat, StorageError>>),
}

async fn storage(
	application: &Application,
	co: &CoId,
	settings: Option<BlockStorageCloneSettings>,
) -> Result<CoStorage, StorageError> {
	match application.co_reducer(&co).await {
		Ok(Some(item)) => Ok(match settings {
			Some(settings) => item.storage().clone_with_settings(settings),
			None => item.storage(),
		}),
		Ok(None) => Err(StorageError::InvalidArgument(anyhow!("Co not found: {}", co))),
		Err(err) => Err(StorageError::InvalidArgument(err)),
	}
}
async fn with_storage<R, F, Fut>(
	application: &Application,
	co: &CoId,
	settings: Option<BlockStorageCloneSettings>,
	f: F,
) -> Result<R, StorageError>
where
	Fut: Future<Output = Result<R, StorageError>>,
	F: FnOnce(CoStorage) -> Fut,
{
	f(storage(application, co, settings).await?).await
}

async fn handle_command(application: &Application, command: Command) {
	match command {
		Command::Get(co, cid, settings, result) => {
			result
				.send(with_storage(application, &co, settings, |storage| async move { storage.get(&cid).await }).await)
				.ok();
		},
		Command::Set(co, block, settings, result) => {
			result
				.send(with_storage(application, &co, settings, |storage| async move { storage.set(block).await }).await)
				.ok();
		},
		Command::Remove(co, cid, settings, result) => {
			result
				.send(
					with_storage(application, &co, settings, |storage| async move { storage.remove(&cid).await }).await,
				)
				.ok();
		},
		Command::Stat(co, cid, settings, result) => {
			result
				.send(with_storage(application, &co, settings, |storage| async move { storage.stat(&cid).await }).await)
				.ok();
		},
	}
}
