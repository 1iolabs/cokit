use crate::CoContext;
use anyhow::anyhow;
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{Block, BlockStorageSettings, CloneWithBlockStorageSettings};
use co_sdk::{Application, BlockStat, BlockStorage, CoId, CoStorage, StorageError};
use dioxus::hooks::use_context;
use futures::Future;
use tokio::sync::{mpsc, oneshot};

pub fn use_co_storage(co: &str) -> CoStorage {
	let (tx, mut rx) = mpsc::unbounded_channel::<Command<<CoBlockStorage as BlockStorage>::StoreParams>>();
	let context: CoContext = use_context();
	context.execute_future_parallel(|application| async move {
		while let Some(command) = rx.recv().await {
			handle_command(&application, command).await;
		}
	});
	CoStorage::new(CoBlockStorage { co: co.into(), tx, settings: None })
}

#[derive(Debug, Clone)]
pub struct CoBlockStorage {
	co: CoId,
	settings: Option<BlockStorageSettings>,
	tx: mpsc::UnboundedSender<Command<<Self as BlockStorage>::StoreParams>>,
}
#[async_trait]
impl BlockStorage for CoBlockStorage {
	type StoreParams = <CoStorage as BlockStorage>::StoreParams;

	/// Returns a block from storage.
	async fn get(&self, cid: &Cid) -> Result<Block<Self::StoreParams>, StorageError> {
		let (result_tx, result_rx) = oneshot::channel();
		self.tx
			.send(Command::Get(self.co.clone(), *cid, self.settings.clone(), result_tx))
			.map_err(|err| StorageError::Internal(err.into()))?;
		result_rx.await.map_err(|err| StorageError::Internal(err.into()))?
	}

	/// Inserts a block into storage.
	/// Returns the CID of the block (gurranted to be the same as the supplied).
	async fn set(&self, block: Block<Self::StoreParams>) -> Result<Cid, StorageError> {
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
}
impl CloneWithBlockStorageSettings for CoBlockStorage {
	fn clone_with_settings(&self, settings: BlockStorageSettings) -> Self {
		CoBlockStorage { co: self.co.clone(), settings: Some(settings), tx: self.tx.clone() }
	}
}

enum Command<P> {
	Get(CoId, Cid, Option<BlockStorageSettings>, oneshot::Sender<Result<Block<P>, StorageError>>),
	Set(CoId, Block<P>, Option<BlockStorageSettings>, oneshot::Sender<Result<Cid, StorageError>>),
	Remove(CoId, Cid, Option<BlockStorageSettings>, oneshot::Sender<Result<(), StorageError>>),
	Stat(CoId, Cid, Option<BlockStorageSettings>, oneshot::Sender<Result<BlockStat, StorageError>>),
}

async fn storage(
	application: &Application,
	co: &CoId,
	settings: Option<BlockStorageSettings>,
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
	settings: Option<BlockStorageSettings>,
	f: F,
) -> Result<R, StorageError>
where
	Fut: Future<Output = Result<R, StorageError>>,
	F: FnOnce(CoStorage) -> Fut,
{
	f(storage(application, co, settings).await?).await
}

async fn handle_command(application: &Application, command: Command<<CoBlockStorage as BlockStorage>::StoreParams>) {
	match command {
		Command::Get(co, cid, settings, result) => {
			result
				.send(with_storage(application, &co, settings, |storage| async move { storage.get(&cid).await }).await)
				.ok();
		},
		Command::Set(co, block, settings, result) => {
			let block = block;
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
