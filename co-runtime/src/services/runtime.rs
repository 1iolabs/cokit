// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{Core, ExecuteError, GuardReference, RuntimeContext, RuntimePool};
use cid::Cid;
#[cfg(not(feature = "js"))]
use co_actor::TaskSpawner;
use co_actor::{ActorError, ActorHandle, Response};
#[cfg(feature = "js")]
use co_actor::{JsLocalTaskSpawner, LocalActor};
use co_primitives::{tags, AnyBlockStorage, CoreBlockStorage, Tags};

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
		self.handle
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
			.map_err(|err| ExecuteError::Other(err.into()))?
	}

	pub async fn execute_guard(
		&self,
		storage: &impl AnyBlockStorage,
		guard_cid: &Cid,
		guard: &GuardReference,
		context: RuntimeContext,
	) -> Result<(RuntimeContext, bool), ExecuteError> {
		self.handle
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
			.map_err(|err| ExecuteError::Other(err.into()))?
	}
}

#[derive(Debug)]
pub enum RuntimeMessage {
	ExecuteState(ExecuteStateAction, Response<Result<RuntimeContext, ExecuteError>>),
	ExecuteGuard(ExecuteGuardAction, Response<Result<(RuntimeContext, bool), ExecuteError>>),
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
		let runtime_service = co_actor::Actor::spawn_with(
			tasks.clone(),
			tags!("type": "runtime", "application": application.into()),
			RuntimeActor::default(),
			tasks,
		)?;
		Ok(RuntimeHandle { handle: runtime_service.handle() })
	}

	#[cfg(feature = "js")]
	pub fn spawn_local(application: impl Into<String>) -> Result<RuntimeHandle, anyhow::Error> {
		let tasks = JsLocalTaskSpawner::default();
		let runtime_service = LocalActor::spawn_with(
			tasks,
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
#[async_trait::async_trait]
impl co_actor::Actor for RuntimeActor {
	type Message = RuntimeMessage;
	type State = (TaskSpawner, RuntimePool);
	type Initialize = TaskSpawner;

	async fn initialize(
		&self,
		_handle: &ActorHandle<Self::Message>,
		_tags: &Tags,
		tasks: Self::Initialize,
	) -> Result<Self::State, ActorError> {
		Ok((tasks.clone(), RuntimePool::new(tasks, Default::default())))
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
				response.spawn_with(spawner.clone(), move || async move {
					state
						.execute_state(&action.storage, &action.core_cid, &action.core, action.context)
						.await
				});
			},
			RuntimeMessage::ExecuteGuard(action, response) => {
				let (spawner, state) = state.clone();
				response.spawn_with(spawner.clone(), move || async move {
					state
						.execute_guard(&action.storage, &action.guard_cid, &action.guard, action.context)
						.await
				});
			},
		}
		Ok(())
	}
}
