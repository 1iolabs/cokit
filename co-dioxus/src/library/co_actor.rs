// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::CoError;
use cid::Cid;
use co_actor::{Actor, ActorError, ActorHandle, Response};
use co_primitives::BlockStorageCloneSettings;
use co_sdk::{
	Block, BlockStat, BlockStorage, CloneWithBlockStorageSettings, CoContext, CoId, CoOptions, CoReducer,
	CoReducerFactory, CoReducerState, CoStorage, StorageError, Tags, TaskSpawner,
};
use dioxus::signals::{SyncSignal, WritableExt};
use futures::{
	future::{select, Either},
	pin_mut, StreamExt,
};
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
		let reducer = match context.try_co_reducer_with_options(&self.id, CoOptions::default()).await {
			Ok(reducer) => {
				// subscribe state and update signal on change
				subscribe_reducer_state(&context, handle, &reducer, signal);
				Some(reducer)
			},
			Err(err) => {
				// surface the error for the current render
				signal.set(Some(Err(CoError::new(err))));

				// subscribe for future unknown memberships
				// 	we do this in a task to be able to detect component unmount
				//  which drops handle
				subscribe_unknown(&context, handle, self.id.clone(), signal);

				None
			},
		};
		Ok(CoActorState { tasks: context.tasks(), reducer })
	}

	/// Handle.
	///
	/// Calls that require a reducer while it's unavailable drop the response
	/// which surfaces as an error on the requester.
	async fn handle(
		&self,
		_handle: &ActorHandle<Self::Message>,
		message: Self::Message,
		state: &mut Self::State,
	) -> Result<(), ActorError> {
		match message {
			CoMessage::SetReducer(reducer) => {
				state.reducer = Some(reducer);
			},
			CoMessage::ReducerState(response) => {
				if let Some(reducer) = &state.reducer {
					response.respond(reducer.reducer_state().await);
				}
			},
			CoMessage::BlockGet(cid, settings, response) => {
				if let Some(reducer) = &state.reducer {
					let storage = storage_with_settings(reducer, settings);
					response.spawn_with(state.tasks.clone(), move || async move { storage.get(&cid).await });
				}
			},
			CoMessage::BlockSet(block, settings, response) => {
				if let Some(reducer) = &state.reducer {
					let storage = storage_with_settings(reducer, settings);
					response.spawn_with(state.tasks.clone(), move || async move { storage.set(block).await });
				}
			},
			CoMessage::BlockStat(cid, settings, response) => {
				if let Some(reducer) = &state.reducer {
					let storage = storage_with_settings(reducer, settings);
					response.spawn_with(state.tasks.clone(), move || async move { storage.stat(&cid).await });
				}
			},
			CoMessage::BlockRemove(cid, settings, response) => {
				if let Some(reducer) = &state.reducer {
					let storage = storage_with_settings(reducer, settings);
					response.spawn_with(state.tasks.clone(), move || async move { storage.remove(&cid).await });
				}
			},
		}
		Ok(())
	}
}

/// Wait for the CO to become available and install its reducer back into the
/// actor via [`CoMessage::SetReducer`], updating the render signal.
fn subscribe_unknown(
	context: &CoContext,
	handle: &ActorHandle<CoMessage>,
	id: CoId,
	mut signal: SyncSignal<Option<Result<CoReducerState, CoError>>>,
) {
	let tasks = context.tasks();
	let context = context.clone();
	let weak_handle = handle.clone().downgrade();
	tasks.spawn(async move {
		let weak_for_closed = weak_handle.clone();
		let open = context.try_co_reducer_with_options(&id, CoOptions::default().with_wait_unknown(None));
		let closed = weak_for_closed.closed();
		pin_mut!(open, closed);
		match select(open, closed).await {
			Either::Left((Ok(reducer), _)) => {
				if let Some(handle) = weak_handle.upgrade() {
					// install the reducer first so subsequent data calls succeed,
					// then start pushing state updates to the render signal.
					let _ = handle.dispatch(CoMessage::SetReducer(reducer.clone()));
					subscribe_reducer_state(&context, &handle, &reducer, signal);
				}
			},
			Either::Left((Err(err), _)) => {
				signal.set(Some(Err(CoError::new(err))));
			},
			// actor closed (component unmounted) - stop without leaking.
			Either::Right(_) => {},
		}
	});
}

fn subscribe_reducer_state(
	context: &CoContext,
	handle: &ActorHandle<CoMessage>,
	reducer: &CoReducer,
	mut signal: SyncSignal<Option<Result<CoReducerState, CoError>>>,
) {
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
}

fn storage_with_settings(reducer: &CoReducer, settings: Option<BlockStorageCloneSettings>) -> CoStorage {
	if let Some(settings) = settings {
		reducer.storage().clone_with_settings(settings)
	} else {
		reducer.storage()
	}
}

pub struct CoActorState {
	tasks: TaskSpawner,
	reducer: Option<CoReducer>,
}

#[derive(Debug)]
pub enum CoMessage {
	/// Install the reducer once the CO becomes available (from the async
	/// open-wait spawned in [`CoActor::initialize`]).
	SetReducer(CoReducer),
	ReducerState(Response<CoReducerState>),
	BlockGet(Cid, Option<BlockStorageCloneSettings>, Response<Result<Block, StorageError>>),
	BlockSet(Block, Option<BlockStorageCloneSettings>, Response<Result<Cid, StorageError>>),
	BlockStat(Cid, Option<BlockStorageCloneSettings>, Response<Result<BlockStat, StorageError>>),
	BlockRemove(Cid, Option<BlockStorageCloneSettings>, Response<Result<(), StorageError>>),
}
