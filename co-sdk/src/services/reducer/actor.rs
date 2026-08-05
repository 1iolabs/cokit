// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use super::{flush::CoReducerFlush, message::ReducerMessage, FlushInfo, ReducerFlushError};
use crate::{
	application::reducer::JoinResult,
	library::{
		extract_next_heads::extract_next_heads,
		log_entries_until::log_entries_until,
		to_external_cid::{to_external_mapped, to_external_mapped_opt},
	},
	reducer::core_resolver::dynamic::DynamicCoreResolver,
	types::{
		co_reducer_context::{CoReducerContextRef, CoReducerFeature},
		co_reducer_state::CoReducerState,
	},
	Action, ApplicationMessage, CoStorage, Reducer, ReducerChangeContext, Runtime,
};
use async_trait::async_trait;
use cid::Cid;
use co_actor::{Actor, ActorError, ActorHandle, ResponseStreams};
use co_identity::{Identity, PrivateIdentityBox};
use co_primitives::{
	BlockLinks, CoId, IgnoreFilter, Link, MappedCid, OptionMappedCid, ReducerAction, Tags, WeakCoReferenceFilter,
};
use co_storage::{BlockStorageContentMapping, BlockStorageExt, OverlayBlockStorage};
use futures::{pin_mut, stream, StreamExt, TryStreamExt};
use indexmap::IndexSet;
use ipld_core::ipld::Ipld;
use std::{
	collections::BTreeSet,
	mem::take,
	sync::{Arc, Mutex},
};

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
	type Initialize =
		(bool, CoStorage, Reducer<CoStorage, DynamicCoreResolver<CoStorage>>, CoReducerFlush, ReducerCache);

	async fn initialize(
		&self,
		_handle: &ActorHandle<Self::Message>,
		_tags: &Tags,
		(initialize, storage, mut reducer, flush, reducer_cache): Self::Initialize,
	) -> Result<Self::State, ActorError> {
		// initialize
		if initialize {
			reducer.initialize(&storage, self.runtime.runtime()).await?;
		}
		reducer_cache.set_reducer_state(CoReducerState::new_reducer(&reducer));

		// state
		let state = ReducerState {
			reducer,
			flush,
			flush_info: None,
			flush_roots: Default::default(),
			network_feature: self.context.has_feature(&CoReducerFeature::Network),
			state_streams: Default::default(),
			reducer_cache,
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
				response.respond(handle_state(state));
			},
			ReducerMessage::StateStream(mut response) => {
				if response.send(CoReducerState::new_reducer(&state.reducer)).is_ok() {
					state.state_streams.push(response);
				}
			},
			ReducerMessage::Push(overlay_storage, storage, identity, action_link, response) => {
				response.respond(handle_push(self, overlay_storage, state, identity, storage, action_link).await);
			},
			ReducerMessage::PushBatch(overlay_storage, storage, identity, memory_state, response) => {
				response
					.respond(handle_push_batch(self, overlay_storage, state, identity, storage, memory_state).await);
			},
			ReducerMessage::JoinState(overlay_storage, storage, join_state, response) => {
				response.respond(handle_join_state(self, overlay_storage, state, storage, join_state).await);
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
	reducer_cache: ReducerCache,
}

#[derive(Debug, Default, Clone)]
pub struct ReducerCache {
	reducer_state: Arc<Mutex<Option<CoReducerState>>>,
}
impl ReducerCache {
	pub fn set_reducer_state(&self, value: CoReducerState) {
		*self.reducer_state.lock().unwrap() = Some(value);
	}

	/// Sometimes we need the reducer state from inside of the reducer and can not wait for queued actor messages.
	/// One example for this is when a internal asks for a block on network.
	/// In this case we need the latest network settings from the CO while the reducer is blocked.
	/// Otherwise this would deadlock.
	pub fn reducer_state(&self) -> Option<CoReducerState> {
		self.reducer_state.lock().unwrap().clone()
	}
}

fn changed(
	reducer_state: &mut ReducerState,
	local: bool,
	identity: Option<&str>,
	roots: impl IntoIterator<Item = CoReducerState>,
) {
	if reducer_state.flush_info.is_none() {
		reducer_state.flush_info = Some(FlushInfo { network: reducer_state.network_feature, ..Default::default() });
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
	CoReducerState::new_reducer(&state.reducer)
}

fn restore_reducer_state(reducer_state: &mut ReducerState, checkpoint: &CoReducerState) {
	reducer_state.reducer.set_reducer_state(checkpoint.state(), checkpoint.heads());
	reducer_state.flush_info = None;
	reducer_state.flush_roots.clear();
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
	let checkpoint = CoReducerState::new_reducer(&reducer_state.reducer);
	let push = match reducer_state
		.reducer
		.push_reference(&storage, actor.runtime.runtime(), &identity, action_link)
		.await
	{
		Ok(push) => push,
		Err(error) => {
			restore_reducer_state(reducer_state, &checkpoint);
			return Err(error);
		},
	};
	let result_state = CoReducerState(push.state, reducer_state.reducer.heads().clone());

	// changed
	changed(reducer_state, true, Some(identity.identity()), [result_state.clone()]);

	// flush
	let committed_error = flush(actor, reducer_state, overlay_storage, &storage, &checkpoint).await?;

	// reactive
	//  load and dispatch as one captured result so a committed flush error can stay the caller-visible one
	let action = storage.get_value(&action_link).await;
	let reactive = match action {
		Ok(action) => actor
			.application_handle
			.dispatch(Action::CoreAction {
				co: actor.id.clone(),
				action,
				storage,
				context: push.context,
				cid: action_link,
				head: push.head,
			})
			.map_err(anyhow::Error::from),
		Err(error) => Err(anyhow::Error::from(error)),
	};

	// result
	match committed_error {
		Some(error) => Err(error),
		None => {
			reactive?;
			Ok(result_state)
		},
	}
}

async fn handle_push_batch(
	actor: &ReducerActor,
	overlay_storage: Option<OverlayBlockStorage<CoStorage>>,
	reducer_state: &mut ReducerState,
	identity: PrivateIdentityBox,
	storage: CoStorage,
	memory_state: CoReducerState,
) -> Result<CoReducerState, anyhow::Error> {
	// save previous heads for reactive dispatch walk and intermediate head walk
	let previous_heads = reducer_state.reducer.heads().clone();

	// derive the heads created by the batch from the log range between
	// `previous_heads` and the new heads; the stream yields newest first, so
	// reverse for chronological (push) order — the flush iterates roots in
	// insertion order, and recording parents before children lets the
	// encryption layer map each one before its child is encrypted; drop the
	// final heads from the list because the trailing roots below already cover
	// them with state, and a duplicate root would cause flush to walk and
	// pin them twice
	let mut intermediate_heads: Vec<Cid> =
		log_entries_until(storage.clone(), memory_state.1.clone(), previous_heads.clone())
			.map_ok(|entry| *entry.cid())
			.try_collect()
			.await?;
	intermediate_heads.reverse();
	intermediate_heads.retain(|intermediate_head| !memory_state.1.contains(intermediate_head));

	// integrate pre-computed state via snapshot + join
	let checkpoint = CoReducerState::new_reducer(&reducer_state.reducer);
	if let Some((state, heads)) = memory_state.some() {
		if let Err(error) = reducer_state.reducer.insert_snapshot(&storage, state, heads).await {
			restore_reducer_state(reducer_state, &checkpoint);
			return Err(error);
		}
	}
	let join_result = match reducer_state
		.reducer
		.join(&storage, &memory_state.1, actor.runtime.runtime())
		.await
	{
		Ok(join_result) => join_result,
		Err(error) => {
			restore_reducer_state(reducer_state, &checkpoint);
			return Err(error.into());
		},
	};

	if join_result.is_some() {
		// changed (local push semantics, not join)
		// each per-push head becomes its own root with no state so the flush
		// promotes its block (and its action block via the link walk) to the
		// next storage, forwards its mapping to the storage core and hands it
		// to ReducerFlush::flush for pinning; intermediate roots are
		// chronological, the final state and memory_state come last
		let intermediate_roots = intermediate_heads
			.iter()
			.map(|intermediate_head| CoReducerState::new(None, BTreeSet::from([*intermediate_head])));
		let trailing_roots = [CoReducerState::new_reducer(&reducer_state.reducer), memory_state];
		changed(reducer_state, true, Some(identity.identity()), intermediate_roots.chain(trailing_roots));
	}

	// capture heads before flush — flush can add pinning batch actions (local.rs)
	// that must not be included in the transaction's reactive dispatch
	let transaction_heads = reducer_state.reducer.heads().clone();

	// flush
	let committed_error = flush(actor, reducer_state, overlay_storage, &storage, &checkpoint).await?;

	// reactive dispatch — walk only transaction entries (pre-flush heads), not flush-generated ones
	let reactive =
		dispatch_actions(actor, storage, transaction_heads, previous_heads, ReducerChangeContext::new()).await;

	// result
	match committed_error {
		Some(error) => Err(error),
		None => {
			reactive?;
			Ok(handle_state(reducer_state))
		},
	}
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
	let checkpoint = CoReducerState::new_reducer(&reducer_state.reducer);
	let join_result = match apply_join(&actor.runtime, reducer_state, &storage, internal_state).await {
		Ok(join_result) => join_result,
		Err(error) => {
			restore_reducer_state(reducer_state, &checkpoint);
			return Err(error);
		},
	};

	// flush
	let committed_error = flush(actor, reducer_state, overlay_storage, &storage, &checkpoint).await?;

	// reactive
	//  walk all actions from previous state to new state and dispatch the actions
	//  we reverse the actions so they arrive with push order (oldest first)
	let mut reactive = Result::<(), anyhow::Error>::Ok(());
	if let Some(join_result) = &join_result {
		// we use the current heads as the flush may applied more actions
		let heads = reducer_state.reducer.heads().clone();
		let previous_heads = join_result.previous_heads.clone();
		reactive = dispatch_actions(actor, storage, heads, previous_heads, ReducerChangeContext::new_join()).await;
	}

	// result
	match committed_error {
		Some(error) => Err(error),
		None => {
			reactive?;
			Ok(handle_state(reducer_state))
		},
	}
}

async fn dispatch_actions(
	actor: &ReducerActor,
	storage: CoStorage,
	heads: BTreeSet<Cid>,
	previous_heads: BTreeSet<Cid>,
	context: ReducerChangeContext,
) -> Result<(), anyhow::Error> {
	let mut actions = log_entries_until(storage.clone(), heads, previous_heads)
		.map(|entry| {
			let storage = storage.clone();
			let context = context.clone();
			async move {
				let entry = entry?;
				let link = entry.entry().payload.into();
				Result::<Action, anyhow::Error>::Ok(Action::CoreAction {
					co: actor.id.clone(),
					action: storage.get_value(&link).await?,
					storage,
					context,
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
	Ok(())
}

async fn flush_before_publication(
	actor: &ReducerActor,
	reducer_state: &mut ReducerState,
	overlay_storage: Option<&OverlayBlockStorage<CoStorage>>,
	storage: &CoStorage,
) -> Result<(Option<FlushInfo>, Option<anyhow::Error>), anyhow::Error> {
	let new_roots = take(&mut reducer_state.flush_roots);

	// log
	tracing::trace!(?new_roots, reducer_state = ?CoReducerState::new_reducer(&reducer_state.reducer), "reducer-flush");

	// base storage
	let base_storage =
		if let Some(overlay_storage) = overlay_storage { overlay_storage.next_storage() } else { storage };

	// flush overlay
	let mut removed_blocks = BTreeSet::<OptionMappedCid>::new();
	if let Some(overlay_storage) = overlay_storage {
		// flush roots from `overlay_storage` to `storage`
		for root in new_roots.iter() {
			// filter links
			// - skip to walk previous head - only use the latest
			// - skip to walk previous state - only use the latest
			// - skip weak references
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
	//  a fatal outcome returns before any publication, a committed one publishes like a success and hands its
	//  error back to the operation handler
	let mut committed_error = None;
	let flush_info = if let Some(flush_info) = reducer_state.flush_info.take() {
		// flush
		match reducer_state
			.flush
			.flush(
				base_storage,
				&mut reducer_state.reducer,
				&flush_info,
				new_roots.into_iter().filter(|root| !root.is_empty()).collect(),
				removed_blocks,
			)
			.await
		{
			Ok(()) => {},
			Err(ReducerFlushError::Fatal(error)) => return Err(error),
			Err(ReducerFlushError::Committed(error)) => committed_error = Some(error),
		}
		Some(flush_info)
	} else {
		None
	};
	Ok((flush_info, committed_error))
}

async fn flush(
	actor: &ReducerActor,
	reducer_state: &mut ReducerState,
	overlay_storage: Option<OverlayBlockStorage<CoStorage>>,
	storage: &CoStorage,
	checkpoint: &CoReducerState,
) -> Result<Option<anyhow::Error>, anyhow::Error> {
	let (flush_info, committed_error) =
		match flush_before_publication(actor, reducer_state, overlay_storage.as_ref(), storage).await {
			Ok(result) => result,
			Err(error) => {
				restore_reducer_state(reducer_state, checkpoint);
				return Err(error);
			},
		};

	if let Some(flush_info) = flush_info {
		// cache
		reducer_state
			.reducer_cache
			.set_reducer_state(CoReducerState::new_reducer(&reducer_state.reducer));

		// notify
		//  once a committed error exists the dispatch result is dropped so it can neither replace that error
		//  nor stop the state below from being published
		let notify = actor
			.application_handle
			.dispatch(Action::CoFlush { co: actor.id.clone(), info: flush_info });
		if committed_error.is_none() {
			notify?;
		}

		// state
		reducer_state
			.state_streams
			.send(CoReducerState::new_reducer(&reducer_state.reducer));
	}
	Ok(committed_error)
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
		reducer_state.reducer.insert_snapshot(storage, state, heads).await?;
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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		application::memory::create_memory_reducer, library::create_reducer_action::create_reducer_action,
		services::reducer::ReducerFlush, Application, ApplicationBuilder, CoReducer, CreateCo, MemoryBlockStorage,
		MonotonicCoUuid, ReducerChangedHandler, CO_CORE_NAME_CO,
	};
	use co_actor::TaskSpawner;
	use co_core_co::CoAction;
	use co_identity::LocalIdentity;
	use co_primitives::{tags, MonotonicCoDate, TagsAction};
	use futures::{FutureExt, Stream};
	use std::{
		collections::VecDeque,
		pin::Pin,
		sync::atomic::{AtomicUsize, Ordering},
	};

	/// Context attached to every simulated hook failure so assertions never depend on error text.
	const TEST_FLUSH_CONTEXT: &str = "test-flush-hook";

	/// Distinct failure type carried by every simulated hook failure.
	#[derive(Debug)]
	struct TestFlushError;
	impl std::fmt::Display for TestFlushError {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			write!(f, "test flush hook failed")
		}
	}
	impl std::error::Error for TestFlushError {}

	/// Distinct failure type carried by the fail-once changed handler.
	#[derive(Debug)]
	struct TestChangedError;
	impl std::fmt::Display for TestChangedError {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			write!(f, "test changed handler failed")
		}
	}
	impl std::error::Error for TestChangedError {}

	struct FailOnceChanged {
		fail: bool,
	}
	#[async_trait]
	impl ReducerChangedHandler<CoStorage, DynamicCoreResolver<CoStorage>> for FailOnceChanged {
		async fn on_state_changed(
			&mut self,
			_storage: &CoStorage,
			_reducer: &Reducer<CoStorage, DynamicCoreResolver<CoStorage>>,
			_context: ReducerChangeContext,
		) -> Result<(), anyhow::Error> {
			if self.fail {
				self.fail = false;
				Err(anyhow::Error::new(TestChangedError))
			} else {
				Ok(())
			}
		}
	}
	type TestChangedHandler = Box<dyn ReducerChangedHandler<CoStorage, DynamicCoreResolver<CoStorage>> + Send + Sync>;
	type TestReducerHooks = (TestFlush, Option<TestChangedHandler>);

	fn test_flush_error() -> anyhow::Error {
		anyhow::Error::new(TestFlushError).context(TEST_FLUSH_CONTEXT)
	}

	/// The three flush outcomes under test. `Fatal` and `Committed` produce the same error object; only the
	/// expectations distinguish them.
	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
	enum FlushCase {
		Success,
		Fatal,
		Committed,
	}

	/// The reducer operations that publish a flush.
	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
	enum Operation {
		Push,
		Batch,
		Join,
	}

	struct TestFlush {
		cases: VecDeque<FlushCase>,
		fatal_roots: Option<IndexSet<CoReducerState>>,
		expect_non_local_after_fatal: bool,
		call_observer: Option<Arc<AtomicUsize>>,
	}
	impl TestFlush {
		fn new(cases: VecDeque<FlushCase>) -> Self {
			Self { cases, fatal_roots: None, expect_non_local_after_fatal: false, call_observer: None }
		}

		fn expecting_non_local_after_fatal(mut self) -> Self {
			self.expect_non_local_after_fatal = true;
			self
		}

		fn observing_calls(mut self, observer: Arc<AtomicUsize>) -> Self {
			self.call_observer = Some(observer);
			self
		}
	}
	#[async_trait]
	impl ReducerFlush<CoStorage, DynamicCoreResolver<CoStorage>> for TestFlush {
		async fn flush(
			&mut self,
			_storage: &CoStorage,
			_reducer: &mut Reducer<CoStorage, DynamicCoreResolver<CoStorage>>,
			info: &FlushInfo,
			new_roots: Vec<CoReducerState>,
			_removed_blocks: BTreeSet<OptionMappedCid>,
		) -> Result<(), ReducerFlushError> {
			if let Some(observer) = &self.call_observer {
				observer.fetch_add(1, Ordering::SeqCst);
			}
			match self.cases.pop_front().unwrap_or(FlushCase::Success) {
				FlushCase::Success => {
					if let Some(fatal_roots) = self.fatal_roots.take() {
						assert!(
							new_roots.iter().all(|root| !fatal_roots.contains(root)),
							"a successful flush must not repeat roots from the fatal attempt"
						);
						if self.expect_non_local_after_fatal {
							assert!(!info.local, "the post-fatal join must remain non-local");
							assert!(
								info.local_identity.is_none(),
								"the post-fatal join must not inherit a local identity"
							);
						}
					}
					Ok(())
				},
				FlushCase::Fatal => {
					self.fatal_roots = Some(new_roots.into_iter().collect());
					Err(ReducerFlushError::Fatal(test_flush_error()))
				},
				FlushCase::Committed => Err(ReducerFlushError::Committed(test_flush_error())),
			}
		}
	}

	/// The target CO's observable application actions, in dispatch order.
	#[derive(Debug, Clone, PartialEq, Eq)]
	enum Observed {
		CoFlush { local: bool, has_local_identity: bool },
		CoreAction(Cid),
	}

	/// Collect every item a stream can yield without blocking, proving nothing extra is already queued.
	fn drain_ready<S>(stream: &mut S) -> Vec<S::Item>
	where
		S: Stream + Unpin,
	{
		let mut items = Vec::new();
		while let Some(Some(item)) = stream.next().now_or_never() {
			items.push(item);
		}
		items
	}

	/// The originating hook error must survive to the caller. The actor request boundary may add its own
	/// context, so walk the chain instead of downcasting the outer error.
	fn assert_test_flush_error(error: &anyhow::Error, label: &str) {
		assert!(
			error.chain().any(|cause| cause.downcast_ref::<TestFlushError>().is_some()),
			"{label}: expected the typed hook error in the chain, got {error:?}"
		);
		assert!(
			error.chain().any(|cause| cause.to_string() == TEST_FLUSH_CONTEXT),
			"{label}: expected the hook context in the chain, got {error:?}"
		);
	}

	fn assert_test_changed_error(error: &anyhow::Error, label: &str) {
		assert!(
			error.chain().any(|cause| cause.downcast_ref::<TestChangedError>().is_some()),
			"{label}: expected the typed changed-handler error in the chain, got {error:?}"
		);
	}

	fn assert_no_test_flush_error(error: &anyhow::Error, label: &str) {
		assert!(
			!error.chain().any(|cause| cause.downcast_ref::<TestFlushError>().is_some()),
			"{label}: expected a dispatch error, got the hook error {error:?}"
		);
	}

	/// A failed dispatch must not reach the caller at all — neither as the returned error nor attached to it.
	/// A closed handle fails with [`ActorError::InvalidState`], so neither that variant nor its wording may
	/// appear anywhere in the chain. The `ActorError::Actor` wrapper that the request boundary always adds is
	/// deliberately not matched.
	fn assert_no_dispatch_error(error: &anyhow::Error, label: &str) {
		assert!(
			!error.chain().any(|cause| {
				let message = cause.to_string();
				matches!(cause.downcast_ref::<ActorError>(), Some(ActorError::InvalidState(..)))
					|| message.contains("Invalid actor state")
					|| message.contains("Actor not running")
			}),
			"{label}: the later dispatch error must not reach the caller, got {error:?}"
		);
	}

	/// Spawn an additional reducer actor for `source`'s CO with a test flush hook. It reuses the CO's real
	/// context, storage, core resolver and date but is not registered in `ReducersControl`.
	async fn spawn_reducer(
		application: &Application,
		source: &CoReducer,
		application_handle: ActorHandle<ApplicationMessage>,
		tasks: &TaskSpawner,
		name: &str,
		state: CoReducerState,
		hooks: TestReducerHooks,
	) -> CoReducer {
		let (flush, changed_handler) = hooks;
		let co = source.id().clone();
		let storage = source.storage();
		let runtime = application.context().inner.runtime();
		let core_resolver = application.context().inner.create_shared_core_resolver(co.clone());
		let mut reducer =
			create_memory_reducer(runtime.runtime(), source.date().clone(), &co, &storage, Some(core_resolver), state)
				.await
				.expect("memory reducer");
		if let Some(changed_handler) = changed_handler {
			reducer.add_change_handler(changed_handler);
		}
		CoReducer::spawn(
			application_handle,
			name.to_owned(),
			co,
			None,
			tasks.clone(),
			runtime,
			reducer,
			source.context.clone(),
			Box::new(flush),
			false,
			None,
		)
		.expect("spawn reducer")
	}

	async fn tags_action(reducer: &CoReducer, identity: &LocalIdentity, tag: &str) -> Link<ReducerAction<Ipld>> {
		create_reducer_action(
			&reducer.storage(),
			identity,
			CO_CORE_NAME_CO,
			&CoAction::Tags { action: TagsAction::insert(tags!("flush-test": tag)) },
			Default::default(),
			reducer.date(),
		)
		.await
		.expect("reducer action")
	}

	/// An incoming branch for join, built on a second success-flush reducer over the same baseline.
	struct Branch {
		first: Cid,
		second: Cid,
		state: CoReducerState,
		_source: CoReducer,
	}

	struct Fixture {
		application: Application,
		identity: LocalIdentity,
		target: CoReducer,
		baseline: CoReducerState,
		cache_baseline: CoReducerState,
		branches: VecDeque<Branch>,
		actions: Pin<Box<dyn Stream<Item = Observed> + Send>>,
		state_stream: Pin<Box<dyn Stream<Item = CoReducerState> + Send>>,
		_source: CoReducer,
		_tasks: TaskSpawner,
	}
	impl Fixture {
		async fn new(
			name: &str,
			flush_cases: VecDeque<FlushCase>,
			closed_application_handle: bool,
			with_branches: bool,
		) -> Self {
			Self::new_with_test_flush(name, TestFlush::new(flush_cases), closed_application_handle, with_branches).await
		}

		async fn new_with_test_flush(
			name: &str,
			flush: TestFlush,
			closed_application_handle: bool,
			with_branches: bool,
		) -> Self {
			Self::new_with_test_flush_and_handler(name, flush, None, closed_application_handle, with_branches).await
		}

		async fn new_with_test_flush_and_handler(
			name: &str,
			flush: TestFlush,
			changed_handler: Option<TestChangedHandler>,
			closed_application_handle: bool,
			with_branches: bool,
		) -> Self {
			co_test::init_test_log();
			let application = ApplicationBuilder::new_memory(name.to_owned())
				.without_keychain()
				.with_disabled_feature("co-local-encryption")
				.with_co_date(MonotonicCoDate::default())
				.with_co_uuid(MonotonicCoUuid::default())
				.build()
				.await
				.expect("application");
			let identity = application.local_identity();
			let source = application
				.create_co(identity.clone(), CreateCo::new(name, None).with_public(true))
				.await
				.expect("create co");
			let base_state = source.reducer_state().await;
			let tasks = TaskSpawner::new(name.to_owned());

			// target actor — only its application handle is closed for the precedence test
			let target_handle =
				if closed_application_handle { ActorHandle::new_closed() } else { application.handle() };
			let target = spawn_reducer(
				&application,
				&source,
				target_handle,
				&tasks,
				name,
				base_state.clone(),
				(flush, changed_handler),
			)
			.await;

			// incoming join branches are prepared before any subscription exists, so their own dispatches are
			// already delivered when the target subscription is installed
			let mut branches = VecDeque::new();
			if with_branches {
				for attempt in 0..2 {
					let branch_name = format!("{name}-branch-{attempt}");
					let branch_source = spawn_reducer(
						&application,
						&source,
						application.handle(),
						&tasks,
						&branch_name,
						base_state.clone(),
						(TestFlush::new([FlushCase::Success].into()), None),
					)
					.await;
					let first_tag = format!("branch-{attempt}-first");
					let second_tag = format!("branch-{attempt}-second");
					let first = tags_action(&branch_source, &identity, &first_tag).await;
					let second = tags_action(&branch_source, &identity, &second_tag).await;
					branch_source.push_reference(&identity, first).await.expect("branch first push");
					branch_source
						.push_reference(&identity, second)
						.await
						.expect("branch second push");
					branches.push_back(Branch {
						first: *first.cid(),
						second: *second.cid(),
						state: branch_source.reducer_state().await,
						_source: branch_source,
					});
				}
			}

			// baselines
			let baseline = target.reducer_state().await;
			let cache_baseline = target.reducer_cache().reducer_state().expect("baseline cache state");

			// pre-subscribed state stream with its immediate baseline item consumed
			let mut state_stream: Pin<Box<dyn Stream<Item = CoReducerState> + Send>> =
				Box::pin(target.reducer_state_stream());
			assert_eq!(state_stream.next().await.expect("baseline state item"), baseline);

			// target-only action stream
			let observed_co = target.id().clone();
			let actions: Pin<Box<dyn Stream<Item = Observed> + Send>> =
				Box::pin(application.actions().filter_map(move |action| {
					let observed_co = observed_co.clone();
					async move {
						match action {
							Action::CoFlush { co, info } if co == observed_co => Some(Observed::CoFlush {
								local: info.local,
								has_local_identity: info.local_identity.is_some(),
							}),
							Action::CoreAction { co, cid, .. } if co == observed_co => {
								Some(Observed::CoreAction(*cid.cid()))
							},
							_ => None,
						}
					}
				}));

			// subscription barrier — the actor handle is FIFO, so a completed request proves the subscription
			// above has been processed
			application
				.handle()
				.request(ApplicationMessage::Context)
				.await
				.expect("subscription barrier");

			Fixture {
				application,
				identity,
				target,
				baseline,
				cache_baseline,
				branches,
				actions,
				state_stream,
				_source: source,
				_tasks: tasks,
			}
		}

		/// Drain the target's dispatched actions after an application barrier.
		async fn drain_actions(&mut self) -> Vec<Observed> {
			self.application
				.handle()
				.request(ApplicationMessage::Context)
				.await
				.expect("action barrier");
			drain_ready(&mut self.actions)
		}
	}

	fn expected_co_flush(operation: Operation) -> Observed {
		match operation {
			Operation::Push | Operation::Batch => Observed::CoFlush { local: true, has_local_identity: true },
			Operation::Join => Observed::CoFlush { local: false, has_local_identity: false },
		}
	}

	/// Run one operation through the real reducer API and return its result with the actions it must dispatch.
	async fn run_operation(
		fixture: &mut Fixture,
		operation: Operation,
		attempt: usize,
	) -> (Result<CoReducerState, anyhow::Error>, Vec<Observed>) {
		match operation {
			Operation::Push => {
				let tag = format!("push-{attempt}");
				let action = tags_action(&fixture.target, &fixture.identity, &tag).await;
				let result = fixture.target.push_reference(&fixture.identity, action).await;
				(result, vec![expected_co_flush(operation), Observed::CoreAction(*action.cid())])
			},
			Operation::Batch => {
				let first_tag = format!("batch-{attempt}-first");
				let second_tag = format!("batch-{attempt}-second");
				let first = tags_action(&fixture.target, &fixture.identity, &first_tag).await;
				let second = tags_action(&fixture.target, &fixture.identity, &second_tag).await;
				let mut transaction = fixture.target.transaction().expect("transaction");
				transaction
					.push_reference(&fixture.identity, first)
					.expect("stage first action");
				transaction
					.push_reference(&fixture.identity, second)
					.expect("stage second action");
				let result = transaction.commit().await;
				(
					result,
					vec![
						expected_co_flush(operation),
						Observed::CoreAction(*first.cid()),
						Observed::CoreAction(*second.cid()),
					],
				)
			},
			Operation::Join => {
				let branch = fixture.branches.pop_front().expect("join branch");
				let expected = vec![
					expected_co_flush(operation),
					Observed::CoreAction(branch.first),
					Observed::CoreAction(branch.second),
				];
				let result = fixture.target.join_state(branch.state.clone()).await;
				(result, expected)
			},
		}
	}

	async fn assert_outcome(
		fixture: &mut Fixture,
		case: FlushCase,
		operation: Operation,
		result: Result<CoReducerState, anyhow::Error>,
		expected_actions: Vec<Observed>,
	) {
		let label = format!("{operation:?}/{case:?}");

		match case {
			FlushCase::Success | FlushCase::Committed => {
				// successful and committed mutations remain applied
				let direct = fixture.target.reducer_state().await;
				assert_ne!(direct, fixture.baseline, "{label}: applied state must differ from the baseline");
				match case {
					FlushCase::Success => {
						assert_eq!(result.expect("success state"), direct, "{label}: returned state")
					},
					FlushCase::Committed => assert_test_flush_error(&result.expect_err("hook error"), &label),
					FlushCase::Fatal => unreachable!(),
				}

				let mut new_stream = Box::pin(fixture.target.reducer_state_stream());
				assert_eq!(new_stream.next().await.expect("new stream item"), direct, "{label}: new stream state");
				assert_eq!(
					fixture.target.reducer_cache().reducer_state().expect("cache state"),
					direct,
					"{label}: cache holds the published state"
				);
				assert_eq!(
					drain_ready(&mut fixture.state_stream),
					vec![direct.clone()],
					"{label}: published exactly once"
				);
				assert_eq!(
					fixture.drain_actions().await,
					expected_actions,
					"{label}: CoFlush before the operation actions"
				);
			},
			FlushCase::Fatal => {
				assert_test_flush_error(&result.expect_err("hook error"), &label);

				// a fatal flush restores actor-visible state before responding
				let direct = fixture.target.reducer_state().await;
				assert_eq!(direct, fixture.baseline, "{label}: direct state stays at the baseline");
				let mut new_stream = Box::pin(fixture.target.reducer_state_stream());
				assert_eq!(
					new_stream.next().await.expect("new stream item"),
					fixture.baseline,
					"{label}: new stream stays at the baseline"
				);
				assert_eq!(
					fixture.target.reducer_cache().reducer_state().expect("cache state"),
					fixture.cache_baseline,
					"{label}: cache stays at the baseline"
				);
				assert!(
					drain_ready(&mut fixture.state_stream).is_empty(),
					"{label}: old stream publishes nothing after the fatal flush"
				);
				let observed = fixture.drain_actions().await;
				assert!(observed.is_empty(), "{label}: nothing dispatched, got {observed:?}");

				// the same actor accepts a distinct successful operation after the fatal attempt
				let (follow_up_result, follow_up_actions) = run_operation(fixture, operation, 1).await;
				assert_ne!(
					follow_up_actions, expected_actions,
					"{label}: follow-up action identifiers must be distinct"
				);
				let follow_up_direct = fixture.target.reducer_state().await;
				assert_ne!(
					follow_up_direct, fixture.baseline,
					"{label}: follow-up state must differ from the baseline"
				);
				assert_eq!(
					follow_up_result.expect("follow-up success state"),
					follow_up_direct,
					"{label}: follow-up returned state"
				);
				assert_eq!(
					fixture.target.reducer_cache().reducer_state().expect("cache state"),
					follow_up_direct,
					"{label}: cache holds the follow-up state"
				);
				assert_eq!(
					drain_ready(&mut fixture.state_stream),
					vec![follow_up_direct.clone()],
					"{label}: old stream publishes the follow-up exactly once"
				);
				assert_eq!(
					drain_ready(&mut new_stream),
					vec![follow_up_direct.clone()],
					"{label}: new stream publishes the follow-up exactly once"
				);
				assert_eq!(
					fixture.drain_actions().await,
					follow_up_actions,
					"{label}: only follow-up actions are dispatched in order"
				);
			},
		}
	}

	async fn assert_operation_matrix(operation: Operation, name: &str) {
		for case in [FlushCase::Success, FlushCase::Fatal, FlushCase::Committed] {
			let mut fixture =
				Fixture::new(&format!("{name}-{case:?}"), [case].into(), false, matches!(operation, Operation::Join))
					.await;
			let (result, expected) = run_operation(&mut fixture, operation, 0).await;
			assert_outcome(&mut fixture, case, operation, result, expected).await;
		}
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn push_publishes_by_flush_outcome() {
		assert_operation_matrix(Operation::Push, "push").await;
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn batch_publishes_by_flush_outcome() {
		assert_operation_matrix(Operation::Batch, "batch").await;
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn join_publishes_by_flush_outcome() {
		assert_operation_matrix(Operation::Join, "join").await;
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn fatal_local_metadata_does_not_leak_into_join() {
		let label = "fatal-push/join-success";
		let flush = TestFlush::new([FlushCase::Fatal].into()).expecting_non_local_after_fatal();
		let mut fixture = Fixture::new_with_test_flush("fatal-local-then-join", flush, false, true).await;

		let (fatal_result, _) = run_operation(&mut fixture, Operation::Push, 0).await;
		assert_test_flush_error(&fatal_result.expect_err("fatal push error"), label);
		assert_eq!(
			fixture.target.reducer_cache().reducer_state().expect("cache state"),
			fixture.cache_baseline,
			"{label}: fatal push leaves the cache at the baseline"
		);
		assert!(drain_ready(&mut fixture.state_stream).is_empty(), "{label}: fatal push publishes no state");
		let fatal_actions = fixture.drain_actions().await;
		assert!(fatal_actions.is_empty(), "{label}: fatal push dispatches no actions, got {fatal_actions:?}");

		let (branch_state, branch_first, branch_second) = {
			let branch = fixture.branches.front().expect("join branch");
			(branch.state.clone(), branch.first, branch.second)
		};
		let storage = fixture.target.storage();
		let runtime = fixture.application.context().inner.runtime();
		let core_resolver = fixture
			.application
			.context()
			.inner
			.create_shared_core_resolver(fixture.target.id().clone());
		let mut clean_reducer = create_memory_reducer(
			runtime.runtime(),
			fixture.target.date().clone(),
			fixture.target.id(),
			&storage,
			Some(core_resolver),
			fixture.baseline.clone(),
		)
		.await
		.expect("clean memory reducer");
		let root_storage = fixture.target.context.storage(false);
		let internal_state = branch_state.to_internal(&root_storage).await;
		if let Some((state, heads)) = internal_state.some() {
			clean_reducer
				.insert_snapshot(&storage, state, heads)
				.await
				.expect("clean join snapshot");
		}
		clean_reducer
			.join(&storage, &internal_state.1, runtime.runtime())
			.await
			.expect("clean join");
		let expected_join_state = CoReducerState::new_reducer(&clean_reducer);
		let expected_join_actions = vec![
			Observed::CoFlush { local: false, has_local_identity: false },
			Observed::CoreAction(branch_first),
			Observed::CoreAction(branch_second),
		];
		let (join_result, _) = run_operation(&mut fixture, Operation::Join, 1).await;
		let joined = join_result.expect("successful join state");
		let direct = fixture.target.reducer_state().await;
		assert_eq!(direct, expected_join_state, "{label}: direct state contains only the successful join");
		let ancestry_action_cids =
			log_entries_until(fixture.target.storage(), direct.1.clone(), fixture.baseline.1.clone())
				.map_ok(|entry| entry.entry().payload)
				.try_collect::<BTreeSet<Cid>>()
				.await
				.expect("join log ancestry");
		assert_eq!(
			ancestry_action_cids,
			BTreeSet::from([branch_first, branch_second]),
			"{label}: restored log ancestry contains only join actions"
		);
		assert_eq!(joined, direct, "{label}: returned join state");
		assert_eq!(
			fixture.target.reducer_cache().reducer_state().expect("cache state"),
			direct,
			"{label}: cache contains only the successful join"
		);
		assert_eq!(
			drain_ready(&mut fixture.state_stream),
			vec![direct.clone()],
			"{label}: old stream publishes only the successful join once"
		);
		assert_eq!(
			fixture.drain_actions().await,
			expected_join_actions,
			"{label}: only non-local join actions are dispatched in order"
		);
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn operation_errors_before_flush_restore_and_recover() {
		for operation in [Operation::Push, Operation::Batch, Operation::Join] {
			let label = format!("changed-handler/{operation:?}");
			let flush_calls = Arc::new(AtomicUsize::new(0));
			let flush = TestFlush::new([FlushCase::Success].into()).observing_calls(flush_calls.clone());
			let changed_handler: TestChangedHandler = Box::new(FailOnceChanged { fail: true });
			let mut fixture = Fixture::new_with_test_flush_and_handler(
				&format!("changed-handler-{operation:?}"),
				flush,
				Some(changed_handler),
				false,
				matches!(operation, Operation::Join),
			)
			.await;

			let (failed_result, failed_actions) = run_operation(&mut fixture, operation, 0).await;
			assert_test_changed_error(&failed_result.expect_err("changed-handler error"), &label);
			assert_eq!(flush_calls.load(Ordering::SeqCst), 0, "{label}: flush hook is not called after the error");

			let direct = fixture.target.reducer_state().await;
			assert_eq!(direct, fixture.baseline, "{label}: direct state stays at the baseline");
			let mut new_stream = Box::pin(fixture.target.reducer_state_stream());
			assert_eq!(
				new_stream.next().await.expect("new stream item"),
				fixture.baseline,
				"{label}: new stream stays at the baseline"
			);
			assert_eq!(
				fixture.target.reducer_cache().reducer_state().expect("cache state"),
				fixture.cache_baseline,
				"{label}: cache stays at the baseline"
			);
			assert!(
				drain_ready(&mut fixture.state_stream).is_empty(),
				"{label}: old stream publishes nothing after the error"
			);
			let observed = fixture.drain_actions().await;
			assert!(observed.is_empty(), "{label}: no actions are dispatched after the error, got {observed:?}");

			let (follow_up_result, follow_up_actions) = run_operation(&mut fixture, operation, 1).await;
			assert_ne!(follow_up_actions, failed_actions, "{label}: follow-up action identifiers must be distinct");
			assert_eq!(flush_calls.load(Ordering::SeqCst), 1, "{label}: only the follow-up reaches flush");
			let follow_up_direct = fixture.target.reducer_state().await;
			assert_ne!(follow_up_direct, fixture.baseline, "{label}: follow-up state must differ from the baseline");
			assert_eq!(
				follow_up_result.expect("follow-up success state"),
				follow_up_direct,
				"{label}: follow-up returned state"
			);
			let ancestry_action_cids =
				log_entries_until(fixture.target.storage(), follow_up_direct.1.clone(), fixture.baseline.1.clone())
					.map_ok(|entry| entry.entry().payload)
					.try_collect::<BTreeSet<Cid>>()
					.await
					.expect("follow-up log ancestry");
			let expected_action_cids = follow_up_actions
				.iter()
				.filter_map(|action| match action {
					Observed::CoreAction(cid) => Some(*cid),
					Observed::CoFlush { .. } => None,
				})
				.collect::<BTreeSet<Cid>>();
			assert_eq!(
				ancestry_action_cids, expected_action_cids,
				"{label}: restored log ancestry contains only follow-up actions"
			);
			assert_eq!(
				fixture.target.reducer_cache().reducer_state().expect("cache state"),
				follow_up_direct,
				"{label}: cache contains only the follow-up state"
			);
			assert_eq!(
				drain_ready(&mut fixture.state_stream),
				vec![follow_up_direct.clone()],
				"{label}: old stream publishes only the follow-up once"
			);
			assert_eq!(
				drain_ready(&mut new_stream),
				vec![follow_up_direct.clone()],
				"{label}: new stream publishes only the follow-up once"
			);
			assert_eq!(
				fixture.drain_actions().await,
				follow_up_actions,
				"{label}: only follow-up actions are dispatched in order"
			);
		}
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn actor_owned_preparation_errors_restore_checkpoint() {
		let name = "actor-owned-preparation-error";
		co_test::init_test_log();
		let application = ApplicationBuilder::new_memory(name.to_owned())
			.without_keychain()
			.with_disabled_feature("co-local-encryption")
			.with_co_date(MonotonicCoDate::default())
			.with_co_uuid(MonotonicCoUuid::default())
			.build()
			.await
			.expect("application");
		let identity = application.local_identity();
		let source = application
			.create_co(identity.clone(), CreateCo::new(name, None).with_public(true))
			.await
			.expect("create co");
		let source_state = source.reducer_state().await;
		let source_storage = source.storage();
		let runtime = application.context().inner.runtime();
		let core_resolver = application.context().inner.create_shared_core_resolver(source.id().clone());
		let mut reducer = create_memory_reducer(
			runtime.runtime(),
			source.date().clone(),
			source.id(),
			&source_storage,
			Some(core_resolver),
			source_state,
		)
		.await
		.expect("memory reducer");
		let checkpoint = CoReducerState::new_reducer(&reducer);

		let absent_head = Cid::default();
		let absent_heads = BTreeSet::from([absent_head]);
		reducer.set_reducer_state(checkpoint.state(), absent_heads.clone());
		assert_ne!(CoReducerState::new_reducer(&reducer), checkpoint, "attempted state differs from checkpoint");

		let flush_calls = Arc::new(AtomicUsize::new(0));
		let reducer_cache = ReducerCache::default();
		reducer_cache.set_reducer_state(checkpoint.clone());
		let mut reducer_state = ReducerState {
			reducer,
			flush: Box::new(TestFlush::new([FlushCase::Success].into()).observing_calls(flush_calls.clone())),
			flush_info: Some(FlushInfo {
				local: true,
				local_identity: Some(identity.identity().to_owned()),
				network: source.context.has_feature(&CoReducerFeature::Network),
			}),
			flush_roots: [CoReducerState::new(checkpoint.state(), absent_heads)].into_iter().collect(),
			network_feature: source.context.has_feature(&CoReducerFeature::Network),
			state_streams: Default::default(),
			reducer_cache: reducer_cache.clone(),
		};
		let actor = ReducerActor::new(source.id().clone(), runtime, application.handle(), source.context.clone());

		let base_storage = CoStorage::new(MemoryBlockStorage::default());
		let temporary_storage = MemoryBlockStorage::default();
		let overlay = OverlayBlockStorage::new(
			TaskSpawner::new(name.to_owned()),
			base_storage,
			temporary_storage,
			None,
			true,
			false,
		);
		let storage = CoStorage::new(overlay.clone());
		let result = flush(&actor, &mut reducer_state, Some(overlay), &storage, &checkpoint).await;

		assert!(result.is_err(), "absent overlay head must fail preparation");
		assert_eq!(flush_calls.load(Ordering::SeqCst), 0, "flush hook must not run after preparation error");
		assert_eq!(
			CoReducerState::new_reducer(&reducer_state.reducer),
			checkpoint,
			"preparation error restores reducer checkpoint"
		);
		assert!(reducer_state.flush_info.is_none(), "preparation error clears flush metadata");
		assert!(reducer_state.flush_roots.is_empty(), "preparation error clears pending roots");
		assert_eq!(
			reducer_cache.reducer_state().expect("cache state"),
			checkpoint,
			"preparation error leaves cache at checkpoint"
		);
	}

	/// A closed application handle makes both the `Action::CoFlush` dispatch and the operation's own action
	/// dispatch fail, which pins down the precedence between a committed hook error and a later error.
	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn closed_application_handle_preserves_flush_error_precedence() {
		// a successful hook keeps the current early return on a failed CoFlush dispatch
		{
			let label = "closed/Success/Push";
			let mut fixture = Fixture::new("closed-success-push", [FlushCase::Success].into(), true, false).await;
			let (result, _) = run_operation(&mut fixture, Operation::Push, 0).await;
			assert_no_test_flush_error(&result.expect_err("dispatch error"), label);
			let direct = fixture.target.reducer_state().await;
			assert_eq!(
				fixture.target.reducer_cache().reducer_state().expect("cache state"),
				direct,
				"{label}: the cache update precedes the CoFlush dispatch"
			);
			assert!(
				drain_ready(&mut fixture.state_stream).is_empty(),
				"{label}: the success path still returns before the state-stream send"
			);
		}

		// a committed hook error survives both the failed CoFlush dispatch and the failed operation dispatch
		for operation in [Operation::Push, Operation::Batch, Operation::Join] {
			let label = format!("closed/Committed/{operation:?}");
			let mut fixture = Fixture::new(
				&format!("closed-committed-{operation:?}"),
				[FlushCase::Committed].into(),
				true,
				matches!(operation, Operation::Join),
			)
			.await;
			let (result, _) = run_operation(&mut fixture, operation, 0).await;
			let error = result.expect_err("committed hook error");
			assert_test_flush_error(&error, &label);
			assert_no_dispatch_error(&error, &label);

			let direct = fixture.target.reducer_state().await;
			assert_ne!(direct, fixture.baseline, "{label}: committed state differs from the baseline");
			assert_eq!(
				fixture.target.reducer_cache().reducer_state().expect("cache state"),
				direct,
				"{label}: the cache holds the committed state"
			);
			assert_eq!(
				drain_ready(&mut fixture.state_stream),
				vec![direct.clone()],
				"{label}: the committed state is published exactly once"
			);
			let mut new_stream = Box::pin(fixture.target.reducer_state_stream());
			assert_eq!(new_stream.next().await.expect("new stream item"), direct, "{label}: new stream state");
		}
	}
}
