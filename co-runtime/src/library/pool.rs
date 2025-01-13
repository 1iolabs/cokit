use crate::{co_v1::CoV1Api, runtimes::RuntimeError, ApiContext, Core, RuntimeContext, RuntimeInstance};
use cid::Cid;
use co_api::Context;
use co_storage::{BlockStorage, StorageError, StoreParamsBlockStorage, SyncBlockStorage};
use std::{collections::VecDeque, sync::Arc};
use tokio::{runtime::Handle, sync::Mutex};

#[derive(Debug)]
pub struct IdleRuntimePool {
	max_runtimes: u16,
	idle: VecDeque<RuntimeInstance>,
}
impl IdleRuntimePool {
	pub fn new(max_runtimes: u16) -> Self {
		Self { max_runtimes, idle: VecDeque::with_capacity(max_runtimes as usize + 1) }
	}

	/// Get an idle runtime for the CID.
	pub fn get(&mut self, cid: &Cid) -> Option<RuntimeInstance> {
		if let Some((index, _)) = self.idle.iter().enumerate().find(|(_, element)| element.cid() == cid) {
			return self.idle.remove(index);
		}
		None
	}

	/// Insert an idle runtime.
	pub fn insert(&mut self, element: RuntimeInstance) {
		self.idle.push_back(element);

		// out pf capacity?
		if self.idle.len() > self.max_runtimes as usize {
			self.idle.pop_front();
		}
	}
}
impl Default for IdleRuntimePool {
	fn default() -> Self {
		IdleRuntimePool::new(8)
	}
}

#[derive(Debug, Clone)]
pub struct RuntimePool {
	pool: Arc<Mutex<IdleRuntimePool>>,
}
impl RuntimePool {
	pub fn new(pool: IdleRuntimePool) -> Self {
		Self { pool: Arc::new(Mutex::new(pool)) }
	}

	pub async fn execute<S>(
		&self,
		storage: &S,
		core: &Core,
		context: RuntimeContext,
	) -> Result<Option<Cid>, ExecuteError>
	where
		S: BlockStorage + Send + Sync + Clone + 'static,
	{
		#[cfg(debug_assertions)]
		let checked = false;
		#[cfg(not(debug_assertions))]
		let checked = false;

		// api
		let api = CoV1Api::new(
			Box::new(SyncBlockStorage::new(StoreParamsBlockStorage::new(storage.clone(), checked), Handle::current())),
			context,
		);

		// execute
		let result = match core {
			Core::Wasm(core) => {
				// get/create instance
				let pool_instance = self.pool.lock().await.get(core);
				let mut instance = match pool_instance {
					Some(i) => i,
					None => RuntimeInstance::create(storage, core).await?,
				};

				// execute
				let (result, instance): (Option<Cid>, RuntimeInstance) =
					tokio::task::spawn_blocking(move || -> Result<(Option<Cid>, RuntimeInstance), RuntimeError> {
						let result = instance.runtime_mut().execute(api)?;
						Ok((result, instance))
					})
					.await
					.map_err(|e| ExecuteError::Other(e.into()))??;

				// pool instance
				self.pool.lock().await.insert(instance);

				// result
				result
			},
			Core::Native(f) => {
				let execute = f.clone();
				tokio::task::spawn_blocking(move || -> Result<Option<Cid>, RuntimeError> {
					let mut context = ApiContext::new(api);
					// Todo: handle panics to not crash the host
					execute(&mut context);
					Ok(context.state())
				})
				.await
				.map_err(|e| ExecuteError::Other(e.into()))??
			},
		};

		// result
		Ok(result)
	}
}
impl Default for RuntimePool {
	fn default() -> Self {
		Self::new(Default::default())
	}
}

#[derive(Debug, thiserror::Error)]
pub enum ExecuteError {
	#[error("Create runtime failed")]
	Create(#[from] StorageError),

	#[error("Execute runtime failed")]
	Runtime(#[from] RuntimeError),

	#[error("Generic runtime error")]
	Other(#[from] anyhow::Error),
}
