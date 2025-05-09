use super::{flush::CoReducerFlush, message::ReducerMessage, FlushInfo};
use crate::{
	library::{
		extract_next_heads::extract_next_heads,
		to_external_cid::{to_external_mapped, to_external_mapped_opt},
		to_internal_cid::to_internal_cids,
	},
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
use co_primitives::{BlockLinks, CoId, Link, MappedCid, OptionMappedCid, ReducerAction, Tags};
use co_storage::{BlockStorage, BlockStorageContentMapping, OverlayBlockStorage, OverlayChange};
use futures::{pin_mut, stream, Stream, StreamExt, TryStreamExt};
use indexmap::IndexSet;
use ipld_core::ipld::Ipld;
use std::{collections::BTreeSet, future::ready, mem::take};
use tokio_stream::wrappers::WatchStream;
use tracing::instrument;

pub struct ReducerActor {
	id: CoId,
	tasks: TaskSpawner,
	runtime: Runtime,
	context: CoReducerContextRef,
}
impl ReducerActor {
	pub fn new(id: CoId, tasks: TaskSpawner, runtime: Runtime, context: CoReducerContextRef) -> Self {
		Self { id, tasks, runtime, context }
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

	#[instrument(err(Debug), skip(self, _handle, state), fields(co = ?self.id))]
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
				response.respond(handle_flush(&self.context, state, overlay_storage, storage).await);
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
	flush_roots: IndexSet<CoReducerState>,
	network_feature: bool,
}

fn changed(
	reducer_state: &mut ReducerState,
	local: bool,
	identity: Option<&str>,
	roots: impl IntoIterator<Item = CoReducerState>,
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
	changed(state, true, Some(identity.identity()), [reducer_state.clone()]);

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
	let internal_state = CoReducerState::new(None, to_internal_cids(internal_storage, heads).await);

	// join
	apply_join(runtime, state, storage, internal_state).await?;

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
	apply_join(runtime, reducer_state, storage, internal_state).await?;

	// result
	Ok(handle_state(reducer_state))
}

async fn handle_flush(
	reducer_context: &CoReducerContextRef,
	reducer_state: &mut ReducerState,
	overlay_storage: Option<OverlayBlockStorage<CoStorage>>,
	storage: CoStorage,
) -> Result<Option<FlushInfo>, anyhow::Error> {
	let new_roots = take(&mut reducer_state.flush_roots);

	// log
	tracing::trace!(?new_roots, reducer_state = ?CoReducerState::new_reducer(&reducer_state.reducer), "reducer-flush");

	// flush overlay
	let mut removed_blocks = BTreeSet::<OptionMappedCid>::new();
	if let Some(overlay_storage) = &overlay_storage {
		// flush roots from `overlay_storage` to `storage`
		for root in new_roots.iter() {
			// skip to walk all head only use the latest
			let links = BlockLinks::default().with_added_ignore(extract_next_heads(overlay_storage, &root.1).await?);

			// flush state
			if let Some(state) = root.0 {
				overlay_storage.flush(state, Some(links.clone())).await?;
			}

			// flush heads
			for head in &root.1 {
				overlay_storage.flush(*head, Some(links.clone())).await?;
			}
		}

		// forward mappings for new roots to base storage
		if storage.is_content_mapped().await {
			let root_storage = reducer_context.storage(true);
			let mappings = stream::iter(new_roots.iter().flat_map(|item| item.iter()))
				.filter_map(|cid| to_external_mapped_opt(&storage, cid))
				.collect::<BTreeSet<MappedCid>>()
				.await;

			// log
			#[cfg(feature = "logging-verbose")]
			tracing::trace!(?mappings, "reducer-flush-mappings");

			// insert
			root_storage.insert_mappings(mappings).await;
		} else {
			#[cfg(feature = "logging-verbose")]
			tracing::trace!("reducer-flush-no-mappings");
		}

		// flush changes
		let changes = overlay_storage.changes();
		pin_mut!(changes);
		while let Some(change) = changes.try_next().await? {
			match change {
				OverlayChange::Set(_cid, _data, _) => {
					// ignore as we only want referenced blocks
					//  this is not "bad" it just indicates that some block got stored which are not used
					//  this also could be intermediate computation inside a core that has later been overwritten

					// log
					#[cfg(feature = "logging-verbose")]
					if co_primitives::MultiCodec::is_cbor(_cid) {
						tracing::warn!(cid = ?_cid, ipld = ?co_primitives::from_cbor::<ipld_core::ipld::Ipld>(&_data), "overlay-unreferenced-block");
					} else {
						tracing::warn!(cid = ?_cid, "overlay-unreferenced-block");
					}
				},
				OverlayChange::Remove(cid) => {
					// record
					removed_blocks.insert(to_external_mapped(&storage, cid).await);

					// remove
					storage.remove(&cid).await?;
				},
			}
		}
	}

	// flush
	if let Some(flush_info) = reducer_state.flush_info.take() {
		reducer_state
			.flush
			.flush(
				&storage,
				&mut reducer_state.reducer,
				new_roots.into_iter().filter(|root| !root.is_empty()).collect(),
				removed_blocks,
			)
			.await?;
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
	reducer_state: &mut ReducerState,
	storage: CoStorage,
	state: CoReducerState,
) -> Result<(), anyhow::Error> {
	// insert snapshot if have state and heads
	if let Some((state, heads)) = state.some() {
		reducer_state.reducer.insert_snapshot(state, heads);
	}

	// join
	if reducer_state.reducer.join(&storage, &state.1, runtime.runtime()).await? {
		// roots
		// - this will include
		// 	 - the latest state
		//     - we dont to flush intermediaries as they are likly not reused and otherwise can be recomputed)
		// 	 - the latest heads that has been loaded and that are linked (not optimal but fine)
		let roots = [CoReducerState::new_reducer(&reducer_state.reducer), state];

		// change
		changed(reducer_state, false, None, roots);
	}
	Ok(())
}
