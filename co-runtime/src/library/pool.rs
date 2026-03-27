// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

#[cfg(feature = "js")]
use crate::library::deferred_storage::DeferredStorage;
use crate::{
	co_v1::CoV1Api, runtimes::RuntimeError, types::guard::GuardReference, Core, RuntimeContext, RuntimeInstance,
};
use cid::Cid;
use co_actor::TaskSpawner;
use co_primitives::{from_cbor, AnyBlockStorage, CoreBlockStorage, GuardInput, ReducerInput};
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
	#[cfg_attr(feature = "js", allow(clippy::arc_with_non_send_sync))]
	pool: Arc<Mutex<IdleRuntimePool>>,
	spawner: TaskSpawner,
}
impl RuntimePool {
	pub fn new(spawner: TaskSpawner, pool: IdleRuntimePool) -> Self {
		#[cfg_attr(feature = "js", allow(clippy::arc_with_non_send_sync))]
		Self { pool: Arc::new(Mutex::new(pool)), spawner }
	}

	fn get_runtime_instance(&self, core: &Cid) -> Option<RuntimeInstance> {
		self.pool.lock().unwrap().get(core)
	}

	fn reuse_runtime_instance(&self, runtime_instance: RuntimeInstance) {
		self.pool.lock().unwrap().insert(runtime_instance);
	}

	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), skip(self, storage), ret)]
	pub async fn execute_state<S>(
		&self,
		storage: &S,
		core_cid: &Cid,
		core: &Core,
		mut context: RuntimeContext,
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
				let (result, instance) =
					execute_with_api(self.spawner.clone(), storage, context, checked, instance, |instance, api| {
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
				let (result, instance) =
					execute_with_api(self.spawner.clone(), storage, context, checked, instance, |instance, api| {
						Ok(instance.runtime_mut().execute_state(api)?)
					})
					.await?;

				// pool instance
				self.reuse_runtime_instance(instance);

				// result
				result
			},
			Core::Native(f) => {
				let reducer_storage = CoreBlockStorage::new(storage.clone(), checked);

				// input
				let reducer_input: ReducerInput =
					from_cbor(&context.input).map_err(|err| ExecuteError::Other(err.into()))?;

				// execute
				let execute = f.clone();
				#[cfg(not(feature = "js"))]
				let reducer_output = self
					.spawner
					.spawn_blocking(Default::default(), move || {
						execute.execute_blocking(reducer_input, reducer_storage)
					})
					.await
					.map_err(|e| ExecuteError::Other(e.into()))?;
				#[cfg(feature = "js")]
				let reducer_output = execute.execute_async(reducer_input, reducer_storage).await;

				// result
				context.apply_reducer_output(reducer_output);
				context
			},
		};

		// result
		Ok(result)
	}

	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), skip(self, storage), ret)]
	pub async fn execute_guard<S>(
		&self,
		storage: &S,
		guard_cid: &Cid,
		guard: &GuardReference,
		mut context: RuntimeContext,
	) -> Result<(RuntimeContext, bool), ExecuteError>
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
				let (result, instance) =
					execute_with_api(self.spawner.clone(), storage, context, checked, instance, |instance, api| {
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
				let (result, instance) = execute_with_api(
					self.spawner.clone(),
					storage,
					context,
					checked,
					instance,
					move |instance, api| Ok(instance.runtime_mut().execute_guard(api)?),
				)
				.await?;

				// pool instance
				self.reuse_runtime_instance(instance);

				// result
				result
			},
			GuardReference::Native(f) => {
				let guard_storage = CoreBlockStorage::new(storage.clone(), checked);

				// input
				let guard_input: GuardInput =
					from_cbor(&context.input).map_err(|err| ExecuteError::Other(err.into()))?;

				// execute
				let guard = f.clone();
				#[cfg(not(feature = "js"))]
				let guard_output = self
					.spawner
					.spawn_blocking(Default::default(), move || guard.execute_blocking(guard_input, guard_storage))
					.await
					.map_err(|e| ExecuteError::Other(e.into()))?;
				#[cfg(feature = "js")]
				let guard_output = guard.execute_async(guard_input, guard_storage).await;

				// output
				let result = guard_output.result;
				context.apply_guard_output(guard_output);
				(context, result)
			},
		};

		// result
		Ok(result)
	}
}
impl Default for RuntimePool {
	fn default() -> Self {
		Self::new(Default::default(), Default::default())
	}
}

#[cfg(not(feature = "js"))]
async fn execute_with_api<T: Send + 'static, I: Send + 'static>(
	spawner: TaskSpawner,
	storage: &impl AnyBlockStorage,
	context: RuntimeContext,
	checked: bool,
	mut instance: I,
	execute: impl Fn(&mut I, CoV1Api) -> Result<T, ExecuteError> + Send + 'static,
) -> Result<(T, I), ExecuteError> {
	// api
	let api = create_cov1_api(storage, context, checked);

	// execute
	let (result, instance) = spawner
		.spawn_blocking(Default::default(), move || (execute(&mut instance, api), instance))
		.await
		.map_err(|e| ExecuteError::Other(e.into()))?;

	// result
	Ok((result?, instance))
}

#[cfg(feature = "js")]
async fn execute_with_api<T: 'static, I: 'static>(
	_spawner: TaskSpawner,
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
