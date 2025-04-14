use super::message::ReducerMessage;
use crate::{
	library::to_internal_cid::to_internal_cids,
	reducer::core_resolver::dynamic::DynamicCoreResolver,
	types::{co_reducer_context::CoReducerContextRef, co_reducer_state::CoReducerState},
	CoStorage, Reducer, Runtime,
};
use async_trait::async_trait;
use cid::Cid;
use co_actor::{Actor, ActorError, ActorHandle, TaskSpawner};
use co_identity::PrivateIdentityBox;
use co_primitives::{Link, ReducerAction, Tags};
use futures::{Stream, StreamExt};
use ipld_core::ipld::Ipld;
use std::{collections::BTreeSet, future::ready};
use tokio_stream::wrappers::WatchStream;

pub struct ReducerActor {
	tasks: TaskSpawner,
	runtime: Runtime,
	context: CoReducerContextRef,
}
impl ReducerActor {
	pub fn new(tasks: TaskSpawner, runtime: Runtime, context: CoReducerContextRef) -> Self {
		Self { tasks, runtime, context }
	}
}
#[async_trait]
impl Actor for ReducerActor {
	type Message = ReducerMessage;
	type State = ReducerState;
	type Initialize = Reducer<CoStorage, DynamicCoreResolver<CoStorage>>;

	async fn initialize(
		&self,
		_handle: &ActorHandle<Self::Message>,
		_tags: &Tags,
		reducer: Self::Initialize,
	) -> Result<Self::State, ActorError> {
		Ok(ReducerState { reducer })
	}

	async fn handle(
		&self,
		_handle: &ActorHandle<Self::Message>,
		message: Self::Message,
		state: &mut Self::State,
	) -> Result<(), ActorError> {
		match message {
			ReducerMessage::State(response) => {
				response.respond(handle_state(&state));
			},
			ReducerMessage::StateStream(response) => {
				let states = handle_state_stream(state);
				// TODO: allow ResponseStream to return an stream directly? (as box?)
				self.tasks.spawn(async move {
					states.map(Ok).forward(response).await.ok();
				});
			},
			ReducerMessage::Push(identity, storage, action_link, response) => {
				response.respond(handle_push(&self.runtime, state, identity, storage, action_link).await);
			},
			ReducerMessage::JoinHeads(storage, heads, response) => {
				response.respond(handle_join(&self.runtime, &self.context.storage(false), state, storage, heads).await);
			},
			ReducerMessage::JoinState(storage, join_state, response) => {
				response.respond(
					handle_join_state(&self.runtime, &self.context.storage(false), state, storage, join_state).await,
				);
			},
			ReducerMessage::Clear(response) => {
				response.respond(handle_clear(state));
			},
		}
		Ok(())
	}
}

pub struct ReducerState {
	reducer: Reducer<CoStorage, DynamicCoreResolver<CoStorage>>,
}

fn handle_state(state: &ReducerState) -> CoReducerState {
	CoReducerState(*state.reducer.state(), state.reducer.heads().clone())
}

fn handle_state_stream(state: &mut ReducerState) -> impl Stream<Item = CoReducerState> {
	WatchStream::new(state.reducer.watch()).filter_map(|state| ready(state.map(CoReducerState::from)))
}

async fn handle_push(
	runtime: &Runtime,
	state: &mut ReducerState,
	identity: PrivateIdentityBox,
	storage: CoStorage,
	action_link: Link<ReducerAction<Ipld>>,
) -> Result<CoReducerState, anyhow::Error> {
	// push
	let reducer_state = CoReducerState(
		state
			.reducer
			.push_reference(&storage, runtime.runtime(), &identity, action_link)
			.await?,
		state.reducer.heads().clone(),
	);

	// result
	Ok(reducer_state)
}

async fn handle_join(
	runtime: &Runtime,
	internal_storage: &CoStorage,
	state: &mut ReducerState,
	storage: CoStorage,
	heads: BTreeSet<Cid>,
) -> Result<CoReducerState, anyhow::Error> {
	// internal
	let internal_heads = to_internal_cids(internal_storage, heads).await;

	// join
	state.reducer.join(&storage, &internal_heads, runtime.runtime()).await?;

	// result
	Ok(handle_state(state))
}

async fn handle_join_state(
	runtime: &Runtime,
	internal_storage: &CoStorage,
	reducer_state: &mut ReducerState,
	storage: CoStorage,
	join_state: CoReducerState,
) -> Result<CoReducerState, anyhow::Error> {
	// internal
	let internal_state = join_state.to_internal(internal_storage).await;

	// join
	if let Some((state, heads)) = internal_state.some() {
		reducer_state.reducer.insert_snapshot(state, heads.clone());
		reducer_state.reducer.join(&storage, &heads, runtime.runtime()).await?;
	}

	// result
	Ok(handle_state(reducer_state))
}

fn handle_clear(reducer_state: &mut ReducerState) -> CoReducerState {
	// clear log
	reducer_state.reducer.log_mut().clear();

	// clear reducer
	reducer_state.reducer.clear();

	// result
	handle_state(reducer_state)
}
