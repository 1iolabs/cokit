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
use co_storage::OverlayBlockStorage;
use futures::{Stream, StreamExt};
use ipld_core::ipld::Ipld;
use std::{collections::BTreeSet, future::ready, mem::take};
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
			flush_roots: Default::default(),
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
			ReducerMessage::Flush(overlay_storage, storage, response) => {
				response.respond(handle_flush(state, overlay_storage, storage).await);
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
	flush_roots: BTreeSet<Cid>,
	network_feature: bool,
}

fn changed(
	reducer_state: &mut ReducerState,
	local: bool,
	identity: Option<&str>,
	roots: impl IntoIterator<Item = Cid>,
) {
	if reducer_state.flush_info.is_none() {
		let mut flush_info = FlushInfo::default();
		flush_info.network = reducer_state.network_feature;
		reducer_state.flush_info = Some(FlushInfo::default());
	}
	if let Some(flush_info) = &mut reducer_state.flush_info {
		if local {
			flush_info.local = true;
			if let Some(identity) = identity {
				flush_info.local_identity = Some(identity.to_owned());
			}
		}
	}
	reducer_state.flush_roots.extend(roots);
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
	changed(state, true, Some(identity.identity()), reducer_state.iter());

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
	apply_join(runtime, state, storage, internal_heads).await?;

	// result
	Ok(handle_state(state))
}

/// See: [`handle_join`]
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
		apply_join(runtime, reducer_state, storage, heads).await?;
	}

	// result
	Ok(handle_state(reducer_state))
}

async fn handle_flush(
	reducer_state: &mut ReducerState,
	overlay_storage: Option<OverlayBlockStorage<CoStorage>>,
	storage: CoStorage,
) -> Result<Option<FlushInfo>, anyhow::Error> {
	// flush overlay
	let roots = take(&mut reducer_state.flush_roots);
	if let Some(overlay_storage) = overlay_storage {
		reducer_state
			.flush
			.flush_overlay(&overlay_storage, roots, &storage, &mut reducer_state.reducer)
			.await?;
	}

	// flush
	if let Some(flush_info) = reducer_state.flush_info.take() {
		reducer_state.flush.flush(&storage, &mut reducer_state.reducer).await?;
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

async fn apply_join(
	runtime: &Runtime,
	state: &mut ReducerState,
	storage: CoStorage,
	internal_heads: BTreeSet<Cid>,
) -> Result<(), anyhow::Error> {
	if state.reducer.join(&storage, &internal_heads, runtime.runtime()).await? {
		// roots
		// - this will include
		// 	 - the latest state
		//     - we dont to flush intermediaries as they are likly not reused and otherwise can be recomputed)
		// 	 - the latest heads that has been loaded and that are linked (not optimal but fine)
		let roots = CoReducerState::new_reducer(&state.reducer);
		let roots = internal_heads.into_iter().chain(roots.iter());

		// change
		changed(state, false, None, roots);
	}
	Ok(())
}
