use super::{flush::CoReducerFlush, message::ReducerMessage, FlushInfo};
use crate::{
	application::reducer::JoinResult,
	library::{
		extract_next_heads::extract_next_heads,
		log_entries_until::log_entries_until,
		to_external_cid::{to_external_mapped, to_external_mapped_opt},
		to_internal_cid::to_internal_cid_opt,
	},
	reducer::core_resolver::dynamic::DynamicCoreResolver,
	types::{
		co_reducer_context::{CoReducerContextRef, CoReducerFeature},
		co_reducer_state::CoReducerState,
	},
	Action, ApplicationMessage, CoStorage, MappedCoReducerState, Reducer, ReducerChangeContext, Runtime,
};
use async_trait::async_trait;
use co_actor::{Actor, ActorError, ActorHandle, ResponseStreams};
use co_identity::{Identity, PrivateIdentityBox};
use co_primitives::{
	BlockLinks, CoId, IgnoreFilter, KnownMultiCodec, Link, MappedCid, MultiCodec, OptionMappedCid, ReducerAction, Tags,
	WeakCoReferenceFilter,
};
use co_storage::{BlockStorageContentMapping, BlockStorageExt, OverlayBlockStorage};
use futures::{pin_mut, stream, StreamExt, TryStreamExt};
use indexmap::IndexSet;
use ipld_core::ipld::Ipld;
use std::{collections::BTreeSet, mem::take};

pub struct ReducerActor {
	id: CoId,
	runtime: Runtime,
	application_handle: ActorHandle<ApplicationMessage>,
	context: CoReducerContextRef,
}
impl ReducerActor {
	pub fn new(
		id: CoId,
		runtime: Runtime,
		application_handle: ActorHandle<ApplicationMessage>,
		context: CoReducerContextRef,
	) -> Self {
		Self { id, runtime, application_handle, context }
	}
}
#[async_trait]
impl Actor for ReducerActor {
	type Message = ReducerMessage;
	type State = ReducerState;
	type Initialize = (bool, CoStorage, Reducer<CoStorage, DynamicCoreResolver<CoStorage>>, CoReducerFlush);

	async fn initialize(
		&self,
		_handle: &ActorHandle<Self::Message>,
		_tags: &Tags,
		(initialize, storage, mut reducer, flush): Self::Initialize,
	) -> Result<Self::State, ActorError> {
		// initialize
		if initialize {
			// check all snapshots are internal
			// note: this will fetch the block from network if neccesarry. To prevent reducer init deadlocks we do this
			//  here to have the actor instance available for caller while doing the network stuff.
			let has_encrypted = reducer
				.snapshots_iter()
				.flat_map(|(state, heads)| [state].into_iter().chain(heads.iter()))
				.any(|cid| MultiCodec::is(cid, KnownMultiCodec::CoEncryptedBlock));
			if has_encrypted {
				let mut internal = Vec::new();
				for (state, heads) in reducer.snapshots_iter() {
					internal.push(CoReducerState::new(Some(*state), heads.clone()).to_internal(&storage).await);
				}
				reducer.clear_snapshots();
				for CoReducerState(state, heads) in internal {
					if let Some(state) = state {
						reducer.insert_snapshot(state, heads);
					}
				}
			}

			// initialize
			reducer.initialize(&storage, self.runtime.runtime()).await?;
		}

		// state
		let state = ReducerState {
			reducer,
			flush,
			flush_info: None,
			flush_roots: Default::default(),
			network_feature: self.context.has_feature(&CoReducerFeature::Network),
			state_streams: Default::default(),
		};

		// result
		Ok(state)
	}

	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), skip(self, _handle, state), fields(co = ?self.id))]
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
			ReducerMessage::StateStream(mut response) => {
				if response.send(CoReducerState::new_reducer(&state.reducer)).is_ok() {
					state.state_streams.push(response);
				}
			},
			ReducerMessage::Push(overlay_storage, storage, identity, action_link, response) => {
				response.respond(handle_push(&self, overlay_storage, state, identity, storage, action_link).await);
			},
			ReducerMessage::JoinState(overlay_storage, storage, join_state, response) => {
				response.respond(handle_join_state(&self, overlay_storage, state, storage, join_state).await);
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
	state_streams: ResponseStreams<CoReducerState>,
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
		reducer_state.flush_info = Some(flush_info);
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

async fn handle_push(
	actor: &ReducerActor,
	overlay_storage: Option<OverlayBlockStorage<CoStorage>>,
	reducer_state: &mut ReducerState,
	identity: PrivateIdentityBox,
	storage: CoStorage,
	action_link: Link<ReducerAction<Ipld>>,
) -> Result<CoReducerState, anyhow::Error> {
	// push
	let push = reducer_state
		.reducer
		.push_reference(&storage, actor.runtime.runtime(), &identity, action_link)
		.await?;
	let result_state = CoReducerState(push.state, reducer_state.reducer.heads().clone());

	// changed
	changed(reducer_state, true, Some(identity.identity()), [result_state.clone()]);

	// flush
	flush(actor, reducer_state, overlay_storage, &storage).await?;

	// reactive
	actor.application_handle.dispatch(Action::CoreAction {
		co: actor.id.clone(),
		action: storage.get_value(&action_link).await?,
		storage,
		context: push.context,
		cid: action_link,
		head: push.head,
	})?;

	// result
	Ok(result_state)
}

/// See: [`handle_join`]
async fn handle_join_state(
	actor: &ReducerActor,
	overlay_storage: Option<OverlayBlockStorage<CoStorage>>,
	reducer_state: &mut ReducerState,
	storage: CoStorage,
	join_state: CoReducerState,
) -> Result<CoReducerState, anyhow::Error> {
	// internal
	let root_storage = actor.context.storage(false);
	let internal_state = join_state.to_internal(&root_storage).await;

	// join
	let join_result = apply_join(&actor.runtime, reducer_state, &storage, internal_state).await?;

	// flush
	flush(actor, reducer_state, overlay_storage, &storage).await?;

	// reactive
	//  walk all actions from previous state to new state and dispatch the actions
	//  we reverse the actions so they arrive with push order (oldest first)
	if let Some(join_result) = &join_result {
		// we use the current heads as the flush may applied more actions
		let heads = reducer_state.reducer.heads().clone();
		let previous_heads = join_result.previous_heads.clone();
		let mut actions = log_entries_until(storage.clone(), heads, previous_heads)
			.map(|entry| {
				let storage = storage.clone();
				async move {
					let entry = entry?;
					let link = entry.entry().payload.into();
					Result::<Action, anyhow::Error>::Ok(Action::CoreAction {
						co: actor.id.clone(),
						action: storage.get_value(&link).await?,
						storage,
						context: ReducerChangeContext::new_join(),
						cid: link,
						head: *entry.cid(),
					})
				}
			})
			.buffered(10)
			.try_collect::<Vec<Action>>()
			.await?;
		actions.reverse();
		for action in actions {
			actor.application_handle.dispatch(action)?;
		}
	}

	// result
	Ok(handle_state(reducer_state))
}

async fn flush(
	actor: &ReducerActor,
	reducer_state: &mut ReducerState,
	overlay_storage: Option<OverlayBlockStorage<CoStorage>>,
	storage: &CoStorage,
) -> Result<(), anyhow::Error> {
	let new_roots = take(&mut reducer_state.flush_roots);

	// log
	tracing::trace!(?new_roots, reducer_state = ?CoReducerState::new_reducer(&reducer_state.reducer), "reducer-flush");

	// base storage
	let base_storage =
		if let Some(overlay_storage) = &overlay_storage { overlay_storage.next_storage() } else { storage };

	// flush overlay
	let mut removed_blocks = BTreeSet::<OptionMappedCid>::new();
	if let Some(overlay_storage) = &overlay_storage {
		// flush roots from `overlay_storage` to `storage`
		for root in new_roots.iter() {
			// skip to walk all head only use the latest
			let links = BlockLinks::default()
				.with_filter(IgnoreFilter::new(extract_next_heads(overlay_storage, &root.1, true).await?))
				.with_filter(WeakCoReferenceFilter::new());

			// flush heads
			for head in &root.1 {
				overlay_storage.flush(*head, Some(links.clone())).await?;
			}

			// flush state
			if let Some(state) = root.0 {
				overlay_storage.flush(state, Some(links.clone())).await?;
			}
		}

		// forward mappings for new roots to base storage
		if base_storage.is_content_mapped().await {
			let root_storage = actor.context.storage(true);
			let mappings = stream::iter(new_roots.iter().flat_map(|item| item.iter()))
				.filter_map(|cid| to_external_mapped_opt(base_storage, cid))
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

		// flush removed
		let changes = overlay_storage.consume_removes();
		pin_mut!(changes);
		while let Some(removed_cid) = changes.try_next().await? {
			removed_blocks.insert(to_external_mapped(base_storage, removed_cid).await);
		}
	}

	// flush
	if let Some(flush_info) = reducer_state.flush_info.take() {
		// flush
		reducer_state
			.flush
			.flush(
				base_storage,
				&mut reducer_state.reducer,
				&flush_info,
				new_roots.into_iter().filter(|root| !root.is_empty()).collect(),
				removed_blocks,
			)
			.await?;

		// notify
		actor
			.application_handle
			.dispatch(Action::CoFlush { co: actor.id.clone(), info: flush_info })?;

		// state
		reducer_state
			.state_streams
			.send(CoReducerState::new_reducer(&reducer_state.reducer));
	}
	Ok(())
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
	storage: &CoStorage,
	state: CoReducerState,
) -> Result<Option<JoinResult>, anyhow::Error> {
	// insert snapshot if have state and heads
	if let Some((state, heads)) = state.some() {
		reducer_state.reducer.insert_snapshot(state, heads);
	}

	// join
	let result = reducer_state.reducer.join(storage, &state.1, runtime.runtime()).await?;
	if let Some(_join_result) = &result {
		// roots
		// - this will include
		// 	 - the latest state
		//     - we dont to flush intermediaries as they are likly not reused and otherwise can be recomputed)
		// 	 - the latest heads that has been loaded and that are linked (not optimal but fine)
		let roots = [CoReducerState::new_reducer(&reducer_state.reducer), state];

		// change
		changed(reducer_state, false, None, roots);
	}
	Ok(result)
}
