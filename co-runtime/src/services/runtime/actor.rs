// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{services::runtime::RuntimeMessage, RuntimeHandle, RuntimePool};
#[cfg(not(feature = "js"))]
use co_actor::TaskSpawner;
use co_actor::{ActorError, ActorHandle};
#[cfg(feature = "js")]
use co_actor::{JsLocalTaskSpawner, LocalActor};
use co_primitives::{tags, Tags};

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
