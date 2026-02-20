#[cfg(feature = "js")]
use crate::services::runtime::js::JsLocalTaskSpawner;
use async_trait::async_trait;
use cid::Cid;
#[cfg(feature = "js")]
use co_actor::LocalActor;
use co_actor::{Actor, ActorError, ActorHandle, Response, TaskSpawner};
use co_primitives::{tags, AnyBlockStorage, CoreBlockStorage, Tags};
use co_runtime::{Core, ExecuteError, GuardReference, RuntimeContext, RuntimePool};

#[derive(Debug, Clone)]
pub struct RuntimeHandle {
	handle: ActorHandle<RuntimeMessage>,
}
impl RuntimeHandle {
	pub async fn execute_state(
		&self,
		storage: &impl AnyBlockStorage,
		core_cid: &Cid,
		core: &Core,
		context: RuntimeContext,
	) -> Result<RuntimeContext, ExecuteError> {
		Ok(self
			.handle
			.request(|response| {
				RuntimeMessage::ExecuteState(
					ExecuteStateAction {
						storage: CoreBlockStorage::new(storage.clone(), false),
						core_cid: *core_cid,
						core: core.clone(),
						context,
					},
					response,
				)
			})
			.await
			.map_err(|err| ExecuteError::Other(err.into()))??)
	}

	pub async fn execute_guard(
		&self,
		storage: &impl AnyBlockStorage,
		guard_cid: &Cid,
		guard: &GuardReference,
		context: RuntimeContext,
	) -> Result<bool, ExecuteError> {
		Ok(self
			.handle
			.request(|response| {
				RuntimeMessage::ExecuteGuard(
					ExecuteGuardAction {
						storage: CoreBlockStorage::new(storage.clone(), false),
						guard_cid: *guard_cid,
						guard: guard.clone(),
						context,
					},
					response,
				)
			})
			.await
			.map_err(|err| ExecuteError::Other(err.into()))??)
	}
}

#[derive(Debug)]
pub enum RuntimeMessage {
	ExecuteState(ExecuteStateAction, Response<Result<RuntimeContext, ExecuteError>>),
	ExecuteGuard(ExecuteGuardAction, Response<Result<bool, ExecuteError>>),
}

#[derive(Debug, Clone)]
pub struct ExecuteStateAction {
	pub storage: CoreBlockStorage,
	pub core_cid: Cid,
	pub core: Core,
	pub context: RuntimeContext,
}

#[derive(Debug, Clone)]
pub struct ExecuteGuardAction {
	pub storage: CoreBlockStorage,
	pub guard_cid: Cid,
	pub guard: GuardReference,
	pub context: RuntimeContext,
}

#[derive(Debug, Default)]
pub struct RuntimeActor {}
impl RuntimeActor {
	#[cfg(not(feature = "js"))]
	pub fn spawn(application: impl Into<String>, tasks: TaskSpawner) -> Result<RuntimeHandle, anyhow::Error> {
		let runtime_service = Actor::spawn_with(
			tasks,
			tags!("type": "runtime", "application": application.into()),
			RuntimeActor::default(),
			(),
		)?;
		Ok(RuntimeHandle { handle: runtime_service.handle() })
	}

	#[cfg(feature = "js")]
	pub fn spawn_local(application: impl Into<String>) -> Result<RuntimeHandle, anyhow::Error> {
		let tasks = JsLocalTaskSpawner::default();
		let runtime_service = LocalActor::spawn_with(
			tasks.clone(),
			tags!("type": "runtime", "application": application.into()),
			RuntimeActor::default(),
			tasks,
		)?;
		Ok(RuntimeHandle { handle: runtime_service.handle() })
	}
}
#[cfg(feature = "js")]
impl LocalActor for RuntimeActor {
	type Message = RuntimeMessage;
	type State = (JsLocalTaskSpawner, RuntimePool);
	type Initialize = JsLocalTaskSpawner;

	async fn initialize(
		&self,
		_handle: &ActorHandle<Self::Message>,
		_tags: &Tags,
		initialize: Self::Initialize,
	) -> Result<Self::State, ActorError> {
		Ok((initialize, RuntimePool::default()))
	}

	async fn handle(
		&self,
		_handle: &ActorHandle<Self::Message>,
		message: Self::Message,
		state: &mut Self::State,
	) -> Result<(), ActorError> {
		match message {
			RuntimeMessage::ExecuteState(action, response) => {
				let (spawner, state) = state.clone();
				response.spawn_local(spawner, move || async move {
					state
						.execute_state(&action.storage, &action.core_cid, &action.core, action.context)
						.await
				});
			},
			RuntimeMessage::ExecuteGuard(action, response) => {
				let (spawner, state) = state.clone();
				response.spawn_local(spawner, move || async move {
					state
						.execute_guard(&action.storage, &action.guard_cid, &action.guard, action.context)
						.await
				});
			},
		}
		Ok(())
	}
}
#[cfg(not(feature = "js"))]
#[async_trait]
impl Actor for RuntimeActor {
	type Message = RuntimeMessage;
	type State = RuntimePool;
	type Initialize = ();

	async fn initialize(
		&self,
		_handle: &ActorHandle<Self::Message>,
		_tags: &Tags,
		_initialize: Self::Initialize,
	) -> Result<Self::State, ActorError> {
		Ok(RuntimePool::default())
	}

	async fn handle(
		&self,
		_handle: &ActorHandle<Self::Message>,
		message: Self::Message,
		state: &mut Self::State,
	) -> Result<(), ActorError> {
		match message {
			RuntimeMessage::ExecuteState(action, response) => {
				let state = state.clone();
				response.spawn(move || async move {
					state
						.execute_state(&action.storage, &action.core_cid, &action.core, action.context)
						.await
				});
			},
			RuntimeMessage::ExecuteGuard(action, response) => {
				let state = state.clone();
				response.spawn(move || async move {
					state
						.execute_guard(&action.storage, &action.guard_cid, &action.guard, action.context)
						.await
				});
			},
		}
		Ok(())
	}
}
