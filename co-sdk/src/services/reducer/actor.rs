use super::{flush::CoReducerFlush, message::ReducerMessage, FlushInfo};
use crate::{
	library::to_internal_cid::to_internal_cids,
	reducer::core_resolver::dynamic::DynamicCoreResolver,
	types::{
		co_reducer_context::{CoReducerContextRef, CoReducerFeature},
		co_reducer_state::CoReducerState,
	},
	CoStorage, Reducer, Runtime,
};
use async_trait::async_trait;
use cid::Cid;
use co_actor::{Actor, ActorError, ActorHandle, TaskSpawner};
use co_identity::{Identity, PrivateIdentityBox};
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
	type Initialize = (Reducer<CoStorage, DynamicCoreResolver<CoStorage>>, CoReducerFlush);

	async fn initialize(
		&self,
		_handle: &ActorHandle<Self::Message>,
		_tags: &Tags,
		(reducer, flush): Self::Initialize,
	) -> Result<Self::State, ActorError> {
		Ok(ReducerState {
			reducer,
			flush,
			flush_info: None,
			network_feature: self.context.has_feature(&CoReducerFeature::Network),
		})
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
			ReducerMessage::Flush(storage, response) => {
				response.respond(handle_flush(state, storage).await);
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
	flush: CoReducerFlush,
	flush_info: Option<FlushInfo>,
	network_feature: bool,
}

fn changed(reducer_state: &mut ReducerState, local: bool, identity: Option<&str>) {
	if reducer_state.flush_info.is_none() {
		let mut flush_info = FlushInfo::default();
		flush_info.network = reducer_state.network_feature;
		reducer_state.flush_info = Some(FlushInfo::default());
	}
	if local {
		if let Some(flush_info) = &mut reducer_state.flush_info {
			flush_info.local = true;
			if let Some(identity) = identity {
				flush_info.local_identity = Some(identity.to_owned());
			}
		}
	}
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

	// changed
	changed(state, true, Some(identity.identity()));

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
	if state.reducer.join(&storage, &internal_heads, runtime.runtime()).await? {
		changed(state, false, None);
	}

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
		if reducer_state.reducer.join(&storage, &heads, runtime.runtime()).await? {
			changed(reducer_state, false, None);
		}
	}

	// result
	Ok(handle_state(reducer_state))
}

async fn handle_flush(
	reducer_state: &mut ReducerState,
	storage: CoStorage,
) -> Result<Option<FlushInfo>, anyhow::Error> {
	if let Some(flush_info) = reducer_state.flush_info.take() {
		reducer_state.flush.flush(&storage, &reducer_state.reducer).await?;
		Ok(Some(flush_info))
	} else {
		Ok(None)
	}
}

fn handle_clear(reducer_state: &mut ReducerState) -> CoReducerState {
	// clear log
	reducer_state.reducer.log_mut().clear();

	// clear reducer
	reducer_state.reducer.clear();

	// result
	handle_state(reducer_state)
}
