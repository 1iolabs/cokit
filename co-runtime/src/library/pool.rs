#[cfg(feature = "js")]
use crate::library::deferred_storage::DeferredStorage;
use crate::{
	co_v1::CoV1Api, runtimes::RuntimeError, types::guard::GuardReference, ApiContext, AsyncContext, Core,
	RuntimeContext, RuntimeInstance,
};
use cid::Cid;
use co_primitives::AnyBlockStorage;
use co_storage::{BlockStorage, StorageError};
use std::{
	collections::VecDeque,
	sync::{Arc, Mutex},
};

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
				let instance = match pool_instance {
					Some(i) => i,
					None => RuntimeInstance::create(storage, core).await?,
				};

				// execute
				let (result, instance) = execute_with_api(storage, context, checked, instance, |instance, api| {
					Ok(instance.runtime_mut().execute_state(api)?)
				})
				.await?;

				// pool instance
				self.reuse_runtime_instance(instance);

				// result
				result
			},
			Core::Binary(bytes) => {
				// get/create instance
				let pool_instance = self.get_runtime_instance(core_cid);
				let instance = match pool_instance {
					Some(i) => i,
					None => RuntimeInstance::create_native(core_cid, bytes).await?,
				};

				// execute
				let (result, instance) = execute_with_api(storage, context, checked, instance, |instance, api| {
					Ok(instance.runtime_mut().execute_state(api)?)
				})
				.await?;

				// pool instance
				self.reuse_runtime_instance(instance);

				// result
				result
			},
			Core::Native(f) => {
				// execute
				let execute = f.clone();
				let (result, _) = execute_with_api(storage, context, checked, (), move |_, api| {
					let mut context = ApiContext::new(api);
					execute(&mut context);
					Ok(context.context().clone())
				})
				.await?;

				// result
				result
			},
			Core::NativeAsync(f) => {
				// api
				let api = AsyncContext::new(storage.clone(), context, checked);

				// execute
				let execute = f.clone();
				#[cfg(not(feature = "js"))]
				let result = tokio::task::spawn_blocking(move || execute.execute_blocking(api).context())
					.await
					.map_err(|e| ExecuteError::Other(e.into()))?;
				#[cfg(feature = "js")]
				let result = execute.execute_async(api).await.context();

				// result
				result
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
				let instance = match pool_instance {
					Some(i) => i,
					None => RuntimeInstance::create(storage, core).await?,
				};

				// execute
				let (result, instance) = execute_with_api(storage, context, checked, instance, |instance, api| {
					Ok(instance.runtime_mut().execute_guard(api)?)
				})
				.await?;

				// pool instance
				self.reuse_runtime_instance(instance);

				// result
				result
			},
			GuardReference::Binary(bytes) => {
				// get/create instance
				let pool_instance = self.get_runtime_instance(guard_cid);
				let instance = match pool_instance {
					Some(i) => i,
					None => RuntimeInstance::create_native(guard_cid, bytes).await?,
				};

				// execute
				let (result, instance) = execute_with_api(storage, context, checked, instance, move |instance, api| {
					Ok(instance.runtime_mut().execute_guard(api)?)
				})
				.await?;

				// pool instance
				self.reuse_runtime_instance(instance);

				// result
				result
			},
			GuardReference::Native(f) => {
				// api
				let api = AsyncContext::new(storage.clone(), context, checked);

				// execute
				let guard = f.clone();
				#[cfg(not(feature = "js"))]
				let result = tokio::task::spawn_blocking(move || guard.execute_blocking(api))
					.await
					.map_err(|e| ExecuteError::Other(e.into()))?;
				#[cfg(feature = "js")]
				let result = guard.execute_async(api).await;

				// result
				result
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

#[cfg(not(feature = "js"))]
async fn execute_with_api<T: Send + 'static, I: Send + 'static>(
	storage: &impl AnyBlockStorage,
	context: RuntimeContext,
	checked: bool,
	mut instance: I,
	execute: impl Fn(&mut I, CoV1Api) -> Result<T, ExecuteError> + Send + 'static,
) -> Result<(T, I), ExecuteError> {
	// api
	let api = create_cov1_api(storage, context, checked);

	// execute
	let (result, instance) = tokio::task::spawn_blocking(move || (execute(&mut instance, api), instance))
		.await
		.map_err(|e| ExecuteError::Other(e.into()))?;

	// result
	Ok((result?, instance))
}

#[cfg(feature = "js")]
async fn execute_with_api<T: 'static, I: 'static>(
	storage: &impl AnyBlockStorage,
	context: RuntimeContext,
	checked: bool,
	mut instance: I,
	execute: impl Fn(&mut I, CoV1Api) -> Result<T, ExecuteError> + 'static,
) -> Result<(T, I), ExecuteError> {
	// api
	let mut api_storage = DeferredStorage::default();
	api_storage.warm(storage, &Default::default(), &context).await?;

	// execute
	loop {
		let api = create_cov1_api(api_storage.clone(), context.clone(), checked);
		match execute(&mut instance, api) {
			Ok(result) => {
				if api_storage.process(storage, false).await? {
					tracing::trace!("deferred-execute-retry");
					continue;
				}
				return Ok((result, instance));
			},
			Err(err) => {
				if api_storage.process(storage, false).await? {
					tracing::trace!(?err, "deferred-execute-retry");
					continue;
				} else {
					tracing::trace!(?err, "deferred-execute-error");
					return Err(err);
				}
			},
		}
	}
}

#[cfg(not(feature = "js"))]
fn create_cov1_api(storage: &impl AnyBlockStorage, context: RuntimeContext, checked: bool) -> CoV1Api {
	CoV1Api::new(
		Box::new(co_storage::SyncBlockStorage::new(
			co_storage::StoreParamsBlockStorage::new(
				storage.clone(),
				checked,
				<co_primitives::DefaultParams as co_primitives::StoreParams>::MAX_BLOCK_SIZE,
			),
			tokio::runtime::Handle::current(),
		)),
		context,
	)
}

#[cfg(feature = "js")]
fn create_cov1_api(storage: DeferredStorage, context: RuntimeContext, _checked: bool) -> CoV1Api {
	CoV1Api::new(Box::new(storage.clone()), context)
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
