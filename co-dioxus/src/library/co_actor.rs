// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::CoError;
use cid::Cid;
use co_actor::{Actor, ActorError, ActorHandle, Response};
use co_primitives::BlockStorageCloneSettings;
use co_sdk::{
	Block, BlockStat, BlockStorage, CloneWithBlockStorageSettings, CoContext, CoId, CoOptions, CoReducer,
	CoReducerFactory, CoReducerState, CoStorage, StorageError, Tags, TaskSpawner,
};
use dioxus::signals::{SyncSignal, WritableExt};
use futures::StreamExt;
use std::future::ready;

pub struct CoActor {
	id: CoId,
}
impl CoActor {
	pub(crate) fn new(id: CoId) -> Self {
		Self { id }
	}
}

#[async_trait::async_trait]
impl Actor for CoActor {
	type Message = CoMessage;
	type State = CoActorState;
	type Initialize = (CoContext, SyncSignal<Option<Result<CoReducerState, CoError>>>);

	async fn initialize(
		&self,
		handle: &ActorHandle<Self::Message>,
		_tags: &Tags,
		(context, mut signal): Self::Initialize,
	) -> Result<Self::State, ActorError> {
		let reducer = match context
			.try_co_reducer_with_options(&self.id, CoOptions::default().with_wait(None))
			.await
		{
			Ok(reducer) => {
				// subscribe state and update signal on change
				context.tasks().spawn({
					let reducer = reducer.clone();
					let weak_handle = handle.clone().downgrade();
					async move {
						reducer
							.reducer_state_stream()
							.take_until(weak_handle.closed())
							.for_each(|reducer_state| {
								signal.set(Some(Ok(reducer_state)));
								ready(())
							})
							.await;
					}
				});

				// result
				reducer
			},
			Err(err) => {
				signal.set(Some(Err(CoError::new(err))));
				return Err(ActorError::Canceled);
			},
		};
		Ok(CoActorState { tasks: context.tasks(), reducer })
	}

	async fn handle(
		&self,
		_handle: &ActorHandle<Self::Message>,
		message: Self::Message,
		state: &mut Self::State,
	) -> Result<(), ActorError> {
		match message {
			CoMessage::ReducerState(response) => {
				response.respond(state.reducer.reducer_state().await);
			},
			CoMessage::BlockGet(cid, settings, response) => {
				response.spawn_with(state.tasks.clone(), {
					let storage = storage_with_settings(state, settings);
					move || async move { storage.get(&cid).await }
				});
			},
			CoMessage::BlockSet(block, settings, response) => {
				response.spawn_with(state.tasks.clone(), {
					let storage = storage_with_settings(state, settings);
					move || async move { storage.set(block).await }
				});
			},
			CoMessage::BlockStat(cid, settings, response) => {
				response.spawn_with(state.tasks.clone(), {
					let storage = storage_with_settings(state, settings);
					move || async move { storage.stat(&cid).await }
				});
			},
			CoMessage::BlockRemove(cid, settings, response) => {
				response.spawn_with(state.tasks.clone(), {
					let storage = storage_with_settings(state, settings);
					move || async move { storage.remove(&cid).await }
				});
			},
		}
		Ok(())
	}
}

fn storage_with_settings(state: &CoActorState, settings: Option<BlockStorageCloneSettings>) -> CoStorage {
	if let Some(settings) = settings {
		state.reducer.storage().clone_with_settings(settings)
	} else {
		state.reducer.storage()
	}
}

pub struct CoActorState {
	tasks: TaskSpawner,
	reducer: CoReducer,
}

#[derive(Debug)]
pub enum CoMessage {
	ReducerState(Response<CoReducerState>),
	BlockGet(Cid, Option<BlockStorageCloneSettings>, Response<Result<Block, StorageError>>),
	BlockSet(Block, Option<BlockStorageCloneSettings>, Response<Result<Cid, StorageError>>),
	BlockStat(Cid, Option<BlockStorageCloneSettings>, Response<Result<BlockStat, StorageError>>),
	BlockRemove(Cid, Option<BlockStorageCloneSettings>, Response<Result<(), StorageError>>),
}
