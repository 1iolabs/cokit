// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{
	co_v1::CoV1Api, runtimes::RuntimeError, types::guard::GuardReference, ApiContext, AsyncContext, Core,
	RuntimeContext, RuntimeInstance,
};
use cid::Cid;
use co_api::{DefaultParams, StoreParams};
use co_storage::{BlockStorage, StorageError, StoreParamsBlockStorage, SyncBlockStorage};
use std::{
	collections::VecDeque,
	sync::{Arc, Mutex},
};
use tokio::runtime::Handle;

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

	fn get_runtime_instance(&self, core: &Cid) -> Option<RuntimeInstance> {
		self.pool.lock().unwrap().get(core)
	}

	fn reuse_runtime_instance(&self, runtime_instance: RuntimeInstance) {
		self.pool.lock().unwrap().insert(runtime_instance);
	}

	pub async fn execute_state<S>(
		&self,
		storage: &S,
		core_cid: &Cid,
		core: &Core,
		context: RuntimeContext,
	) -> Result<RuntimeContext, ExecuteError>
	where
		S: BlockStorage + Send + Sync + Clone + 'static,
	{
		#[cfg(debug_assertions)]
		let checked = false;
		#[cfg(not(debug_assertions))]
		let checked = false;

		// execute
		let result = match core {
			Core::Wasm(core) => {
				// get/create instance
				let pool_instance = self.get_runtime_instance(core);
				let mut instance = match pool_instance {
					Some(i) => i,
					None => RuntimeInstance::create(storage, core).await?,
				};

				// api
				let api = create_cov1_api(storage, context, checked);

				// execute
				let (result, instance): (RuntimeContext, RuntimeInstance) =
					tokio::task::spawn_blocking(move || -> Result<(RuntimeContext, RuntimeInstance), RuntimeError> {
						let result = instance.runtime_mut().execute_state(api)?;
						Ok((result, instance))
					})
					.await
					.map_err(|e| ExecuteError::Other(e.into()))??;

				// pool instance
				self.reuse_runtime_instance(instance);

				// result
				result
			},
			Core::Binary(bytes) => {
				// get/create instance
				let pool_instance = self.get_runtime_instance(core_cid);
				let mut instance = match pool_instance {
					Some(i) => i,
					None => RuntimeInstance::create_native(core_cid, bytes).await?,
				};

				// api
				let api = create_cov1_api(storage, context, checked);

				// execute
				let (result, instance): (RuntimeContext, RuntimeInstance) =
					tokio::task::spawn_blocking(move || -> Result<(RuntimeContext, RuntimeInstance), RuntimeError> {
						let result = instance.runtime_mut().execute_state(api)?;
						Ok((result, instance))
					})
					.await
					.map_err(|e| ExecuteError::Other(e.into()))??;

				// pool instance
				self.reuse_runtime_instance(instance);

				// result
				result
			},
			Core::Native(f) => {
				// api
				let api = create_cov1_api(storage, context, checked);

				// execute
				let execute = f.clone();
				tokio::task::spawn_blocking(move || -> Result<RuntimeContext, RuntimeError> {
					let mut context = ApiContext::new(api);
					// Todo: handle panics to not crash the host
					execute(&mut context);
					Ok(context.context().clone())
				})
				.await
				.map_err(|e| ExecuteError::Other(e.into()))??
			},
			Core::NativeAsync(f) => {
				// api
				let api = AsyncContext::new(storage.clone(), context, checked);

				// execute
				let execute = f.clone();
				tokio::task::spawn_blocking(move || -> Result<RuntimeContext, RuntimeError> {
					// Todo: handle panics to not crash the host
					Ok(execute(api).context())
				})
				.await
				.map_err(|e| ExecuteError::Other(e.into()))??
			},
		};

		// result
		Ok(result)
	}

	pub async fn execute_guard<S>(
		&self,
		storage: &S,
		guard_cid: &Cid,
		guard: &GuardReference,
		context: RuntimeContext,
	) -> Result<bool, ExecuteError>
	where
		S: BlockStorage + Send + Sync + Clone + 'static,
	{
		#[cfg(debug_assertions)]
		let checked = false;
		#[cfg(not(debug_assertions))]
		let checked = false;

		// execute
		let result = match guard {
			GuardReference::Wasm(core) => {
				// get/create instance
				let pool_instance = self.get_runtime_instance(core);
				let mut instance = match pool_instance {
					Some(i) => i,
					None => RuntimeInstance::create(storage, core).await?,
				};

				// api
				let api = create_cov1_api(storage, context, checked);

				// execute
				let (result, instance) =
					tokio::task::spawn_blocking(move || -> Result<(bool, RuntimeInstance), RuntimeError> {
						let result = instance.runtime_mut().execute_guard(api)?;
						Ok((result, instance))
					})
					.await
					.map_err(|e| ExecuteError::Other(e.into()))??;

				// pool instance
				self.reuse_runtime_instance(instance);

				// result
				result
			},
			GuardReference::Binary(bytes) => {
				// get/create instance
				let pool_instance = self.get_runtime_instance(guard_cid);
				let mut instance = match pool_instance {
					Some(i) => i,
					None => RuntimeInstance::create_native(guard_cid, bytes).await?,
				};

				// api
				let api = create_cov1_api(storage, context, checked);

				// execute
				let (result, instance) =
					tokio::task::spawn_blocking(move || -> Result<(bool, RuntimeInstance), RuntimeError> {
						let result = instance.runtime_mut().execute_guard(api)?;
						Ok((result, instance))
					})
					.await
					.map_err(|e| ExecuteError::Other(e.into()))??;

				// pool instance
				self.reuse_runtime_instance(instance);

				// result
				result
			},
			GuardReference::Native(f) => {
				// api
				let api = AsyncContext::new(storage.clone(), context, checked);

				// execute
				let execute = f.clone();
				tokio::task::spawn_blocking(move || -> Result<bool, RuntimeError> {
					// Todo: handle panics to not crash the host
					let result = execute(api);
					Ok(result)
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

fn create_cov1_api<S: BlockStorage + Clone + 'static>(storage: &S, context: RuntimeContext, checked: bool) -> CoV1Api {
	CoV1Api::new(
		Box::new(SyncBlockStorage::new(
			StoreParamsBlockStorage::new(storage.clone(), checked, DefaultParams::MAX_BLOCK_SIZE),
			Handle::current(),
		)),
		context,
	)
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
