use crate::{co_v1::CoV1Api, runtimes::RuntimeError, RuntimeInstance};
use co_storage::{BlockStorage, StorageError};
use libipld::Cid;
use std::{collections::VecDeque, sync::Arc};
use tokio::sync::Mutex;

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
		return None
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

	pub async fn execute<S>(&self, storage: &S, cid: &Cid, api: CoV1Api) -> Result<Option<Cid>, SharedRuntimePoolError>
	where
		S: BlockStorage + Send,
	{
		// get/create instance
		let pool_instance = self.pool.lock().await.get(cid);
		let mut instance = match pool_instance {
			Some(i) => i,
			None => RuntimeInstance::create(storage, cid).await?,
		};

		// execute
		let (result, instance): (Option<Cid>, RuntimeInstance) =
			tokio::task::spawn_blocking(move || -> Result<(Option<Cid>, RuntimeInstance), RuntimeError> {
				let result = instance.runtime_mut().execute(api)?;
				Ok((result, instance))
			})
			.await
			.map_err(|e| SharedRuntimePoolError::Other(e.into()))??;

		// pool instance
		self.pool.lock().await.insert(instance);

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
pub enum SharedRuntimePoolError {
	#[error("Storage error")]
	Storage(#[from] StorageError),

	#[error("Runtime error")]
	Runtime(#[from] RuntimeError),

	#[error("Error")]
	Other(#[from] anyhow::Error),
}
