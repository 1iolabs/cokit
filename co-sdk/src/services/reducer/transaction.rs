// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{
	application::memory::create_memory_reducer,
	library::create_reducer_action::{create_reducer_action, store_reducer_action},
	reducer::core_resolver::dynamic::DynamicCoreResolver,
	services::reducer::message::ReducerMessage,
	types::co_reducer_state::CoReducerState,
	CoReducer, CoStorage, Reducer, Runtime,
};
use async_trait::async_trait;
use co_actor::{Actor, ActorError, ActorHandle, Response};
use co_identity::PrivateIdentityBox;
use co_primitives::{CoId, DynamicCoDate, Link, ReducerAction};
use ipld_core::ipld::Ipld;
use serde::Serialize;
use std::fmt::Debug;

pub(super) struct TransactionActor {
	runtime: Runtime,
	identity: PrivateIdentityBox,
}
impl TransactionActor {
	pub fn new(runtime: Runtime, identity: PrivateIdentityBox) -> Self {
		Self { runtime, identity }
	}
}

#[derive(Debug)]
pub(super) enum TransactionMessage {
	/// Push action reference to memory reducer (fire-and-forget).
	Push(Link<ReducerAction<Ipld>>),
	/// Return final state or first error.
	Commit(Response<Result<CoReducerState, anyhow::Error>>),
}

pub(super) struct TransactionState {
	memory_reducer: Reducer<CoStorage, DynamicCoreResolver<CoStorage>>,
	storage: CoStorage,
	error: Option<anyhow::Error>,
}

#[async_trait]
impl Actor for TransactionActor {
	type Message = TransactionMessage;
	type State = TransactionState;
	type Initialize = (ActorHandle<ReducerMessage>, CoStorage, DynamicCoreResolver<CoStorage>, DynamicCoDate, CoId);

	async fn initialize(
		&self,
		_handle: &ActorHandle<Self::Message>,
		_tags: &co_primitives::Tags,
		(reducer_handle, storage, core_resolver, date, id): Self::Initialize,
	) -> Result<Self::State, ActorError> {
		// fetch current state from main reducer actor
		let current_state = reducer_handle.request(ReducerMessage::State).await?;

		// create memory reducer (shares storage, isolated state/heads)
		let memory_reducer =
			create_memory_reducer(self.runtime.runtime(), date, &id, &storage, Some(core_resolver), current_state)
				.await?;

		Ok(TransactionState { memory_reducer, storage, error: None })
	}

	async fn handle(
		&self,
		_handle: &ActorHandle<Self::Message>,
		message: Self::Message,
		state: &mut Self::State,
	) -> Result<(), ActorError> {
		match message {
			TransactionMessage::Push(action_link) => {
				// skip if a previous push already failed
				if state.error.is_none() {
					if let Err(err) = state
						.memory_reducer
						.push_reference(&state.storage, self.runtime.runtime(), &self.identity, action_link)
						.await
					{
						state.error = Some(err);
					}
				}
			},
			TransactionMessage::Commit(response) => {
				if let Some(err) = state.error.take() {
					response.respond(Err(err));
				} else {
					response.respond(Ok(CoReducerState::new_reducer(&state.memory_reducer)));
				}
			},
		}
		Ok(())
	}
}

/// Batches multiple push operations into a single flush.
///
/// A background actor processes actions in parallel with the caller.
/// Transactions are atomic — if any action fails, none are applied.
pub struct CoReducerTransaction {
	pub(super) reducer: CoReducer,
	pub(super) identity: PrivateIdentityBox,
	pub(super) handle: ActorHandle<TransactionMessage>,
	pub(super) count: usize,
}
impl CoReducerTransaction {
	/// Stage an action without flushing.
	pub async fn push<A>(&mut self, core: impl Into<String> + Debug, item: &A) -> Result<(), anyhow::Error>
	where
		A: Serialize + Debug + Clone + Send + Sync + 'static,
	{
		let action_reference = create_reducer_action(
			&self.reducer.storage(),
			&self.identity,
			core,
			item,
			Default::default(),
			self.reducer.date(),
		)
		.await?;
		self.handle.dispatch(TransactionMessage::Push(action_reference))?;
		self.count += 1;
		Ok(())
	}

	/// Stage a pre-constructed action without flushing.
	pub async fn push_action<A>(&mut self, action: &ReducerAction<A>) -> Result<(), anyhow::Error>
	where
		A: Serialize + Debug + Send + Sync + Clone + 'static,
	{
		let action_reference = store_reducer_action(&self.reducer.storage(), action, Default::default()).await?;
		self.handle.dispatch(TransactionMessage::Push(action_reference))?;
		self.count += 1;
		Ok(())
	}

	/// Stage an already-stored action reference.
	pub fn push_reference(&mut self, action_reference: Link<ReducerAction<Ipld>>) -> Result<(), anyhow::Error> {
		self.handle.dispatch(TransactionMessage::Push(action_reference))?;
		self.count += 1;
		Ok(())
	}

	/// Commit all staged actions and flush in one step.
	pub async fn commit(self) -> Result<CoReducerState, anyhow::Error> {
		if self.count == 0 {
			return Ok(self.reducer.reducer_state().await);
		}

		// get pre-computed state from transaction actor
		let memory_state = self.handle.try_request(TransactionMessage::Commit).await?;

		// integrate into main reducer
		self.reducer.commit_transaction(self.identity, memory_state).await
	}
}
