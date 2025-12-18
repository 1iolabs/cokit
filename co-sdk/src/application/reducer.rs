use crate::{
	library::create_reducer_action::create_reducer_action,
	reducer::state_resolver::{
		DynamicStateResolver, JoinStateResolver, StateResolver, StateResolverContext, StaticStateResolver,
	},
	CoDate, CoreResolver, CoreResolverContext, DynamicCoDate,
};
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use cid::Cid;
use co_identity::PrivateIdentity;
use co_log::{EntryBlock, Log, LogError};
use co_primitives::{Link, ReducerAction, SignedEntry};
use co_runtime::RuntimePool;
use co_storage::{BlockStorageExt, ExtendedBlockStorage};
use futures::{pin_mut, stream, StreamExt, TryStreamExt};
use ipld_core::ipld::Ipld;
use serde::Serialize;
use std::{
	collections::{BTreeSet, HashMap, VecDeque},
	fmt::{Debug, Formatter},
	marker::PhantomData,
	mem::swap,
};
use tokio::sync::watch;

pub struct ReducerBuilder<S, R> {
	_storage: PhantomData<S>,
	/// Log.
	log: Log,
	/// The core resolver which composes the state.
	core_resolver: R,
	/// Latest state.
	state: Option<Cid>,
	/// Latests heads.
	heads: BTreeSet<Cid>,
	/// Avilable historic snapshots.
	snapshots: HashMap<BTreeSet<Cid>, Cid>,
	/// Initialize
	initialize: bool,
	/// State resolvers
	state_resolver: Vec<DynamicStateResolver<S>>,
}
impl<S, R> ReducerBuilder<S, R>
where
	S: ExtendedBlockStorage + Send + Sync + Clone + 'static,
	R: CoreResolver<S> + Send + Sync + 'static,
{
	pub fn new(core_resolver: R, log: Log) -> Self {
		Self {
			_storage: PhantomData,
			core_resolver,
			heads: Default::default(),
			snapshots: Default::default(),
			state: None,
			log,
			initialize: true,
			state_resolver: Default::default(),
		}
	}

	pub fn core_resolver_mut(&mut self) -> &mut R {
		&mut self.core_resolver
	}

	pub fn with_initialize(self, initialize: bool) -> Self {
		Self { initialize, ..self }
	}

	/// Use state/heads as lastest.
	/// This will also disable the initialize because we know the latest state already.
	pub fn with_latest_state(self, state: Cid, heads: BTreeSet<Cid>) -> Self {
		Self { state: Some(state), heads, initialize: false, ..self }
	}

	pub fn with_snapshot(self, state: Cid, heads: BTreeSet<Cid>) -> Self {
		let mut snapshots = self.snapshots;
		snapshots.insert(heads, state);
		Self { snapshots, ..self }
	}

	pub fn with_state_resolver(mut self, state_resolver: impl StateResolver<S>) -> Self {
		self.state_resolver.push(DynamicStateResolver::new(state_resolver));
		self
	}

	pub async fn build(
		self,
		storage: &S,
		runtime: &RuntimePool,
		date: impl CoDate,
	) -> Result<Reducer<S, R>, anyhow::Error> {
		// validate heads
		if self.state.is_some() && self.log.heads() != &self.heads {
			return Err(anyhow!("Invalid heads. The log and state heads must be the same"));
		}

		// static state resolver
		let state_resolver = if !self.snapshots.is_empty() {
			StaticStateResolver::new_unsorted(
				storage,
				stream::iter(self.snapshots).map(|(heads, state)| (state, heads)),
			)
			.await?
		} else {
			StaticStateResolver::default()
		};

		// state resolvers
		let state_resolver = if !self.state_resolver.is_empty() {
			DynamicStateResolver::new(
				self.state_resolver
					.into_iter()
					.fold(JoinStateResolver::new(state_resolver), JoinStateResolver::join),
			)
		} else {
			DynamicStateResolver::new(state_resolver)
		};

		// build
		let mut result = Reducer {
			core_resolver: self.core_resolver,
			heads: self.heads,
			state: self.state,
			log: self.log,
			change_handlers: Default::default(),
			watch: watch::channel(None),
			date: date.boxed(),
			state_resolver,
		};
		if self.initialize {
			result.initialize(storage, runtime).await?;
		}
		Ok(result)
	}
}

/// The reducers combines the log to a root state.
pub struct Reducer<S, R> {
	/// Storage.
	log: Log,
	/// The core resolver which composes the state.
	core_resolver: R,
	/// Latest state.
	state: Option<Cid>,
	/// Latest heads.
	heads: BTreeSet<Cid>,
	/// Change handlers.
	change_handlers: Vec<Box<dyn ReducerChangedHandler<S, R>>>,
	/// State/Heads watcher.
	watch: (watch::Sender<Option<(Cid, BTreeSet<Cid>)>>, watch::Receiver<Option<(Cid, BTreeSet<Cid>)>>),
	/// Date.
	date: DynamicCoDate,
	/// State resolver.
	state_resolver: DynamicStateResolver<S>,
}
impl<S, R> Reducer<S, R>
where
	S: ExtendedBlockStorage + Send + Sync + Clone + 'static,
	R: CoreResolver<S> + Send + Sync + 'static,
{
	/// Initialize this reducer by computing current state if one.
	#[tracing::instrument(level = tracing::Level::TRACE, skip(self, storage, runtime))]
	pub async fn initialize(&mut self, storage: &S, runtime: &RuntimePool) -> Result<(), anyhow::Error> {
		tracing::trace!(?self.state_resolver, "reducer-initialize");
		let context = ReducerChangeContext { cause: ReducerChangeCause::Initialize };

		// initialize state resolver
		self.state_resolver.initialize(storage).await?;

		// if we have snapshots but no state/heads join all heads from snapshots
		// find latest state if we have snapshots but no latest selection
		if self.state.is_none() && self.heads.is_empty() {
			// provide roots
			let context = StateResolverContext::default();
			if let Some(roots) = self.state_resolver.provide_roots(storage, &context) {
				for (state, heads) in roots.try_collect::<Vec<_>>().await? {
					// join heads
					self.log.join_heads(storage, heads.iter()).await?;

					// use state?
					//  do this every iteration so we end up with the latest known state
					if state.is_some() && self.log.heads() == &heads {
						self.state = state;
						self.heads = heads;
					}
				}
				tracing::trace!(state = ?self.state, heads = ?self.heads, log_heads = ?self.log.heads(), "reducer-roots");
			}
		}

		// if log heads are different from reducer heads
		if &self.heads != self.log.heads() {
			// compute the state
			let (state, heads) = self.compute_state(storage, runtime, &context).await?;
			self.state = state;
			self.heads = heads;
		}

		// fail if we have state but no heads
		if self.state.is_some() && self.heads.is_empty() {
			return Err(anyhow!("State but no heads"));
		}

		// fail if we have heads but no state
		if self.state.is_none() && !self.heads.is_empty() {
			return Err(anyhow!("Heads but no state"));
		}

		// notify
		self.on_state_changed(storage, &context).await?;

		// log
		tracing::trace!(?self.state, ?self.heads, "reducer-initialized");

		// if we have state and heads we are fine
		Ok(())
	}

	/// (Re)sets the reducer and the log to a given state.
	pub fn set_reducer_state(&mut self, state: Option<Cid>, heads: BTreeSet<Cid>) {
		self.log.set_heads(heads.clone());
		self.state = state;
		self.heads = heads;
	}

	pub fn into_log(self) -> Log {
		self.log
	}

	pub fn date(&self) -> &DynamicCoDate {
		&self.date
	}

	pub fn is_empty(&self) -> bool {
		self.heads.is_empty() && self.state.is_none()
	}

	/// Get state observable.
	pub fn watch(&self) -> watch::Receiver<Option<(Cid, BTreeSet<Cid>)>> {
		self.watch.1.clone()
	}

	/// Add change handler which will be called when state changed.
	/// Change handlers will be called in the sequence they have been added.
	pub fn add_change_handler(&mut self, handler: Box<dyn ReducerChangedHandler<S, R> + Send + Sync>) {
		self.change_handlers.push(handler);
	}

	/// CoreResolver.
	pub fn core_resolver(&self) -> &R {
		&self.core_resolver
	}

	/// Mutable CoreResolver.
	pub fn core_resolver_mut(&mut self) -> &mut R {
		&mut self.core_resolver
	}

	/// Latest state.
	pub fn state(&self) -> &Option<Cid> {
		&self.state
	}

	/// Latest heads.
	pub fn heads(&self) -> &BTreeSet<Cid> {
		&self.heads
	}

	/// The log.
	pub fn log(&self) -> &Log {
		&self.log
	}

	/// The log.
	pub fn log_mut(&mut self) -> &mut Log {
		&mut self.log
	}

	/// Clear all state but latest.
	pub fn clear(&mut self) {
		self.clear_snapshots();
	}

	/// Clear all state but latest.
	pub fn clear_snapshots(&mut self) {
		self.state_resolver.clear();
	}

	/// Insert previous snapshots (trusted) of the same log from which we can continue.
	pub async fn insert_snapshot(
		&mut self,
		storage: &S,
		state: Cid,
		heads: BTreeSet<Cid>,
	) -> Result<(), anyhow::Error> {
		self.state_resolver.push_state(storage, state, heads).await?;
		Ok(())
	}

	/// Find the state matching the specified heads, if one.
	async fn find_state(
		&self,
		storage: &S,
		context: &StateResolverContext,
		heads: &BTreeSet<Cid>,
	) -> Result<Option<(Cid, BTreeSet<Cid>)>, anyhow::Error> {
		if &self.heads == heads {
			if let Some(state) = self.state {
				return Ok(Some((state, self.heads.clone())));
			}
		}
		if let Some((resolved_state, resolved_heads)) =
			self.state_resolver.resolve_state(storage, &context, heads).await?
		{
			if &resolved_heads == heads {
				return Ok(Some((resolved_state, resolved_heads)));
			}
		}
		Ok(None)
	}

	/// Push an event.
	///
	/// # Returns
	/// The resulting state.
	pub async fn push<T, I>(
		&mut self,
		storage: &S,
		runtime: &RuntimePool,
		identity: &I,
		core: impl Into<String> + Debug,
		item: &T,
	) -> Result<PushResult, anyhow::Error>
	where
		T: Serialize + Send + Sync,
		I: PrivateIdentity + Send + Sync,
	{
		self.push_reference(
			storage,
			runtime,
			identity,
			create_reducer_action(storage, identity, core, item, Default::default(), &self.date).await?,
		)
		.await
	}

	/// Push an event.
	///
	/// # Returns
	/// The resulting state.
	pub async fn push_action<T, I>(
		&mut self,
		storage: &S,
		runtime: &RuntimePool,
		identity: &I,
		action: &ReducerAction<T>,
	) -> Result<PushResult, anyhow::Error>
	where
		T: Serialize + Send + Sync,
		I: PrivateIdentity + Send + Sync,
	{
		let action_link = storage.set_serialized(&action).await?.into();
		self.push_reference(storage, runtime, identity, action_link).await
	}

	/// Push an event.
	///
	/// # Returns
	/// The resulting state.
	pub async fn push_reference<I>(
		&mut self,
		storage: &S,
		runtime: &RuntimePool,
		identity: &I,
		action_link: Link<ReducerAction<Ipld>>,
	) -> Result<PushResult, anyhow::Error>
	where
		I: PrivateIdentity + Send + Sync,
	{
		self.push_reference_with_state(storage, runtime, identity, action_link, None)
			.await
	}

	/// Push an event.
	///
	/// # Returns
	/// The resulting state.
	///
	/// # Note
	/// Specifing a `core_state_link` may is dangerous and the caller is responsible to know that:
	/// `action_link + current core state = core_state_link`.
	pub async fn push_reference_with_state<I>(
		&mut self,
		storage: &S,
		runtime: &RuntimePool,
		identity: &I,
		action_link: Link<ReducerAction<Ipld>>,
		core_state_link: Option<Cid>,
	) -> Result<PushResult, anyhow::Error>
	where
		I: PrivateIdentity + Send + Sync,
	{
		let action: ReducerAction<serde::de::IgnoredAny> = storage.get_deserialized(action_link.as_ref()).await?;

		// validate
		if identity.identity() != action.from {
			return Err(anyhow!("Invalid argument: identity"));
		}

		// apply to log
		let entry = self
			.log
			.push(storage, identity, *action_link.cid())
			.await
			.with_context(|| format!("push event core: {}: {:?}", action.core, action_link))?;

		// // debug
		// let block = self.log.storage().get(entry.as_ref()).await.unwrap();
		// let ipld: Ipld = IpldCodec::DagCbor.decode(block.data()).unwrap();
		// println!("entry = {:?}", ipld);

		// apply to state
		let context = CoreResolverContext {
			change: ReducerChangeContext { cause: ReducerChangeCause::Push },
			entry,
			state: core_state_link,
		};
		let runtime_context = self
			.core_resolver
			.execute(storage, runtime, &context, &self.state, action_link.cid())
			.await
			.with_context(|| {
				format!(
					"runtime execute core: {}, state: {:?}, action: {:?}, head: {:?}",
					action.core,
					self.state,
					action_link,
					context.entry.cid(),
					// to_json_string(&action.payload)
				)
			})?;

		// log
		#[cfg(feature = "logging-verbose")]
		{
			tracing::trace!(
				co = self.log.id_string(),
				previous_state = ?self.state,
				head = ?context.entry.cid(),
				next_state = ?runtime_context.state,
				"compute-state-push",
			);
		}

		// fail and ignore result when we got a failure disgnostic
		//  this is technically optional because its fine to have failing transactions
		//  which just have no effect to the state
		//  but in case of push which is always local we can just skip it
		//  it makes no sense to propagate it to peers etc.
		runtime_context.ok(storage).await?;

		// snapshot
		if self.state.is_some() {
			self.insert_snapshot(storage, self.state.unwrap(), self.heads.clone()).await?;
		}

		// update
		self.state = runtime_context.state;
		self.heads = self.log.heads_iter().cloned().collect();

		// notify
		self.on_state_changed(storage, &context.change).await?;

		// result
		Ok(PushResult {
			entry: context.entry.cid().into(),
			context: context.change,
			head: runtime_context.event,
			state: runtime_context.state,
		})
	}

	/// Join heads (from other log).
	/// This is used to join logs from other peers.
	/// Returns `Some(JoinResult)` if new heads has integrated.
	pub async fn join(
		&mut self,
		storage: &S,
		heads: &BTreeSet<Cid>,
		runtime: &RuntimePool,
	) -> Result<Option<JoinResult>, LogError> {
		// join
		let log_heads = self.log().heads().clone();
		if &log_heads != heads && (self.log_mut().join_heads(storage, heads.iter()).await? || self.heads != log_heads) {
			let context = ReducerChangeContext { cause: ReducerChangeCause::Log };

			// sync state
			let state_context = StateResolverContext { state: self.state, heads: self.heads.clone() };
			let (has_snapshot, (next_state, next_heads)) =
				if let Some((state, heads)) = self.find_state(storage, &state_context, self.log().heads()).await? {
					// use snapshot
					(true, (Some(state), heads))
				} else {
					// compute state
					(false, self.compute_state(storage, runtime, &context).await?)
				};

			// result
			let result = JoinResult {
				context: context.clone(),
				heads: next_heads.clone(),
				previous_heads: self.heads.clone(),
				state: next_state,
				previous_state: self.state,
			};

			// apply
			if next_state != self.state || self.heads != next_heads {
				// log
				tracing::trace!(
					co = self.log().id_string(),
					log_heads = ?if log_heads == self.heads { None } else { Some(&log_heads) },
					join_heads = ?heads,
					?result,
					has_snapshot,
					"joined"
				);

				// apply
				self.state = next_state;
				self.heads = next_heads;

				// notify
				self.on_state_changed(storage, &context).await?;
			}
			Ok(Some(result))
		} else {
			Ok(None)
		}
	}

	/// Notify subscribers about change.
	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), skip(self, storage))]
	async fn on_state_changed(&mut self, storage: &S, context: &ReducerChangeContext) -> Result<(), LogError> {
		// handlers
		//  run sequencial in same order to not have non deterministic results
		let mut change_handlers = Vec::new();
		swap(&mut change_handlers, &mut self.change_handlers);
		let mut last_result: Result<(), anyhow::Error> = Ok(());
		for change_handler in change_handlers.iter_mut() {
			last_result = change_handler
				.on_state_changed(storage, &self, context.clone())
				.await
				.with_context(|| format!("running {:?}", change_handler.type_name()));
			if last_result.is_err() {
				break;
			}
		}
		swap(&mut change_handlers, &mut self.change_handlers);
		last_result?;

		// watch
		if let Some(state) = self.state {
			self.watch
				.0
				.send(Some((state, self.heads.clone())))
				.expect("watcher not dropped before reducer");
		}

		// result
		Ok(())
	}

	/// Compute state for log heads.
	/// Returns the resulting state if one.
	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), skip(self, runtime, storage))]
	async fn compute_state(
		&self,
		storage: &S,
		runtime: &RuntimePool,
		context: &ReducerChangeContext,
	) -> Result<(Option<Cid>, BTreeSet<Cid>), anyhow::Error> {
		// compute stack
		let (mut state, stack) = self.compute_stack(storage).await?;

		// apply stack
		for entry in stack {
			let _previous_state = state;

			// context
			let action = entry.entry().payload;
			let context = CoreResolverContext { change: context.clone(), entry, state: None };

			// apply
			state = self
				.core_resolver
				.execute(storage, runtime, &context, &state, &action)
				.await?
				.state;

			// log
			#[cfg(feature = "logging-verbose")]
			{
				tracing::trace!(
					co = self.log.id_string(),
					previous_state = ?_previous_state,
					head = ?context.entry.cid(),
					next_state = ?state,
					"compute-state-join",
				);
			}
		}

		// result
		Ok((state, self.log.heads().clone()))
	}

	/// Compute stack to apply to an state.
	/// The computed start position is self.heads.
	/// The computed end position is self.log.heads.
	/// Algorithm: We search for the lowest known ancestors of the heads while walking the log backwards.
	#[tracing::instrument(level = tracing::Level::TRACE, skip(self, storage))]
	async fn compute_stack(&self, storage: &S) -> Result<(Option<Cid>, VecDeque<EntryBlock>), anyhow::Error> {
		let context = StateResolverContext { state: self.state, heads: self.heads.clone() };
		let heads: BTreeSet<Cid> = self.log.heads().clone();
		let mut state = None;
		let mut stack = VecDeque::new();

		// is latest state?
		if self.heads != heads {
			// find latest usable historic state
			let entries = self.log.stream(storage);
			pin_mut!(entries);
			let mut current_heads = heads.clone();
			while let Some(entry) = entries.try_next().await? {
				// update current_heads to reflect the heads without this entry
				current_heads.remove(entry.cid());
				current_heads.extend(entry.entry().next.iter().cloned());

				// put on stack to reapply
				stack.push_front(entry);

				// does the current heads reference a state we know?
				// note: this will never match if we have no previous states
				//  and a new state will be recomputed from scratch
				if let Some((entry_state, _)) = self.find_state(storage, &context, &current_heads).await? {
					state = Some(entry_state);
					break;
				}
			}
		}

		// result
		Ok((state, stack))
	}
}

#[derive(Debug, Clone)]
pub struct PushResult {
	/// The resuting state.
	pub state: Option<Cid>,
	/// The latest head after the push operation.
	pub head: Cid,
	/// The entry reference. Normally the same as the head (if not changed by an CoreResolver).
	pub entry: Link<SignedEntry>,
	/// The change context.
	pub context: ReducerChangeContext,
}

#[derive(Debug, Clone)]
pub struct JoinResult {
	/// The change context.
	pub context: ReducerChangeContext,

	/// The resuting state.
	pub state: Option<Cid>,
	/// The latest head after the join operation.
	pub heads: BTreeSet<Cid>,

	/// The state before the join operation.
	pub previous_state: Option<Cid>,
	/// The heads before the join operation.
	pub previous_heads: BTreeSet<Cid>,
}

/// Reducer change handler.
/// Will be executed everytime the state in the reducer changes, including on initialize.
#[async_trait]
pub trait ReducerChangedHandler<S, R>: Send + Sync {
	async fn on_state_changed(
		&mut self,
		storage: &S,
		reducer: &Reducer<S, R>,
		context: ReducerChangeContext,
	) -> Result<(), anyhow::Error>;

	/// Diagnostic.
	fn type_name(&self) -> String {
		std::any::type_name::<Self>().to_owned()
	}
}
impl<S, R> Debug for Reducer<S, R> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Reducer")
			.field("id", &self.log.id_string())
			.field("state", &self.state)
			.field("heads", &self.heads)
			.field("state_resolver", &self.state_resolver)
			.finish()
	}
}

#[derive(Debug, Clone)]
pub struct ReducerChangeContext {
	cause: ReducerChangeCause,
}
impl ReducerChangeContext {
	/// Create a new local change context.
	pub fn new() -> Self {
		Self { cause: ReducerChangeCause::Push }
	}

	/// Create a new local change context.
	pub fn new_join() -> Self {
		Self { cause: ReducerChangeCause::Log }
	}

	/// Whether this change was caused locally.
	pub fn is_local_change(&self) -> bool {
		self.cause.is_local()
	}
}

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
enum ReducerChangeCause {
	/// Change caused by reducer initialization.
	Initialize,
	/// Change caused by an log operation (join).
	Log,
	/// Change caused by local push operation.
	Push,
}
impl ReducerChangeCause {
	/// Whether this change was caused locally.
	pub fn is_local(&self) -> bool {
		match self {
			ReducerChangeCause::Push => true,
			_ => false,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::Reducer;
	use crate::{
		application::reducer::ReducerBuilder, build_core, crate_repository_path, CoDate, CoreResolver, MonotonicCoDate,
		ReducerChangeContext, ReducerChangedHandler, SingleCoreResolver,
	};
	use async_trait::async_trait;
	use cid::Cid;
	use co_identity::{IdentityResolverBox, LocalIdentityResolver};
	use co_log::{IdentityEntryVerifier, Log};
	use co_primitives::{BlockSerializer, ReducerAction};
	use co_runtime::{Core, IdleRuntimePool, RuntimePool};
	use co_storage::{ExtendedBlockStorage, MemoryBlockStorage};
	use example_counter::{Counter, CounterAction};
	use futures::StreamExt;

	#[tokio::test]
	async fn smoke() {
		// store
		let storage = MemoryBlockStorage::default();

		// wasm
		let repository_path = crate_repository_path(true).unwrap();
		let core_path = repository_path.join("examples/counter");
		let wasm = build_core(repository_path, core_path)
			.unwrap()
			.store_artifact(&storage)
			.await
			.unwrap();

		// logs
		let identity1 = LocalIdentityResolver::default().private_identity("did:local:p1").unwrap();
		let identity2 = LocalIdentityResolver::default().private_identity("did:local:p2").unwrap();
		let identity3 = LocalIdentityResolver::default().private_identity("did:local:p3").unwrap();
		let log1 = Log::new(
			"test".as_bytes().to_vec(),
			IdentityEntryVerifier::new(IdentityResolverBox::new(LocalIdentityResolver::default())),
			Default::default(),
		);
		let log2 = Log::new(
			"test".as_bytes().to_vec(),
			IdentityEntryVerifier::new(IdentityResolverBox::new(LocalIdentityResolver::default())),
			Default::default(),
		);
		let log3 = Log::new(
			"test".as_bytes().to_vec(),
			IdentityEntryVerifier::new(IdentityResolverBox::new(LocalIdentityResolver::default())),
			Default::default(),
		);

		// pool
		let date = MonotonicCoDate::default().boxed();
		let runtime = RuntimePool::new(IdleRuntimePool::default());
		let core_resolver = SingleCoreResolver::new(wasm, wasm.into());
		let native_core_resolver = SingleCoreResolver::new(wasm, Core::native::<Counter>());

		// reducer
		let mut reducer1 = ReducerBuilder::new(native_core_resolver, log1)
			.build(&storage, &runtime, date.clone())
			.await
			.unwrap();
		let mut reducer2 = ReducerBuilder::new(core_resolver.clone(), log2)
			.build(&storage, &runtime, date.clone())
			.await
			.unwrap();
		let mut reducer3 = ReducerBuilder::new(core_resolver.clone(), log3)
			.build(&storage, &runtime, date.clone())
			.await
			.unwrap();

		// 1-6
		reducer1
			.push(&storage, &runtime, &identity1, "test", &CounterAction::Increment(1))
			.await
			.unwrap();
		reducer1
			.push(&storage, &runtime, &identity1, "test", &CounterAction::Increment(2))
			.await
			.unwrap();
		reducer1
			.push(&storage, &runtime, &identity1, "test", &CounterAction::Increment(3))
			.await
			.unwrap();
		reducer1
			.push(&storage, &runtime, &identity1, "test", &CounterAction::Increment(4))
			.await
			.unwrap();
		reducer1
			.push(&storage, &runtime, &identity1, "test", &CounterAction::Increment(5))
			.await
			.unwrap();
		reducer1
			.push(&storage, &runtime, &identity1, "test", &CounterAction::Increment(6))
			.await
			.unwrap();
		reducer2.join(&storage, reducer1.heads(), &runtime).await.unwrap();
		reducer3.join(&storage, reducer1.heads(), &runtime).await.unwrap();
		assert_eq!(21, counter_state(&storage, &reducer1).await.0); // 1 + 2 + 3 + 4 + 5 + 6
		assert_eq!(21, counter_state(&storage, &reducer2).await.0);
		assert_eq!(21, counter_state(&storage, &reducer3).await.0);

		// 7
		reducer2
			.push(&storage, &runtime, &identity2, "test", &CounterAction::Increment(7))
			.await
			.unwrap();
		reducer3.join(&storage, reducer2.heads(), &runtime).await.unwrap();
		reducer1.join(&storage, reducer3.heads(), &runtime).await.unwrap();
		assert_eq!(28, counter_state(&storage, &reducer1).await.0);
		assert_eq!(28, counter_state(&storage, &reducer2).await.0);
		assert_eq!(28, counter_state(&storage, &reducer3).await.0);

		// 8
		reducer3
			.push(&storage, &runtime, &identity3, "test", &CounterAction::Increment(8))
			.await
			.unwrap();
		reducer2.join(&storage, reducer3.heads(), &runtime).await.unwrap();
		reducer1.join(&storage, reducer2.heads(), &runtime).await.unwrap();
		assert_eq!(36, counter_state(&storage, &reducer1).await.0);
		assert_eq!(36, counter_state(&storage, &reducer2).await.0);
		assert_eq!(36, counter_state(&storage, &reducer3).await.0);

		// 9
		reducer3
			.push(&storage, &runtime, &identity3, "test", &CounterAction::Increment(9))
			.await
			.unwrap();
		reducer2.join(&storage, reducer3.heads(), &runtime).await.unwrap();
		reducer1.join(&storage, reducer2.heads(), &runtime).await.unwrap();
		assert_eq!(45, counter_state(&storage, &reducer1).await.0);
		assert_eq!(45, counter_state(&storage, &reducer2).await.0);
		assert_eq!(45, counter_state(&storage, &reducer3).await.0);

		// A
		reducer1
			.push(&storage, &runtime, &identity1, "test", &CounterAction::Increment(10))
			.await
			.unwrap();
		reducer2.join(&storage, reducer1.heads(), &runtime).await.unwrap();
		reducer3.join(&storage, reducer2.heads(), &runtime).await.unwrap();
		assert_eq!(55, counter_state(&storage, &reducer1).await.0);
		assert_eq!(55, counter_state(&storage, &reducer2).await.0);
		assert_eq!(55, counter_state(&storage, &reducer3).await.0);

		// B
		reducer1
			.push(&storage, &runtime, &identity1, "test", &CounterAction::Set(11))
			.await
			.unwrap();
		reducer2.join(&storage, reducer1.heads(), &runtime).await.unwrap();
		assert_eq!(11, counter_state(&storage, &reducer1).await.0);
		assert_eq!(11, counter_state(&storage, &reducer2).await.0);
		assert_eq!(55, counter_state(&storage, &reducer3).await.0);

		// C
		reducer1
			.push(&storage, &runtime, &identity1, "test", &CounterAction::Increment(12))
			.await
			.unwrap();
		reducer2
			.push(&storage, &runtime, &identity2, "test", &CounterAction::Increment(12))
			.await
			.unwrap();
		reducer2.join(&storage, reducer1.heads(), &runtime).await.unwrap();
		reducer1.join(&storage, reducer2.heads(), &runtime).await.unwrap();
		assert_eq!(35, counter_state(&storage, &reducer1).await.0);
		assert_eq!(35, counter_state(&storage, &reducer2).await.0);
		assert_eq!(55, counter_state(&storage, &reducer3).await.0);

		// D
		reducer1
			.push(&storage, &runtime, &identity1, "test", &CounterAction::Increment(13))
			.await
			.unwrap();
		reducer2.join(&storage, reducer1.heads(), &runtime).await.unwrap();
		assert_eq!(48, counter_state(&storage, &reducer1).await.0);
		assert_eq!(48, counter_state(&storage, &reducer2).await.0);
		assert_eq!(55, counter_state(&storage, &reducer3).await.0);

		// E
		reducer2
			.push(&storage, &runtime, &identity2, "test", &CounterAction::Increment(14))
			.await
			.unwrap();
		reducer1.join(&storage, reducer2.heads(), &runtime).await.unwrap();
		assert_eq!(62, counter_state(&storage, &reducer1).await.0);
		assert_eq!(62, counter_state(&storage, &reducer2).await.0);
		assert_eq!(55, counter_state(&storage, &reducer3).await.0);

		// B*
		reducer3
			.push(&storage, &runtime, &identity3, "test", &CounterAction::Increment(11))
			.await
			.unwrap();
		reducer3.join(&storage, reducer1.heads(), &runtime).await.unwrap();
		reducer2.join(&storage, reducer3.heads(), &runtime).await.unwrap();
		reducer1.join(&storage, reducer2.heads(), &runtime).await.unwrap();
		assert_eq!(73, counter_state(&storage, &reducer1).await.0);
		assert_eq!(73, counter_state(&storage, &reducer2).await.0);
		assert_eq!(73, counter_state(&storage, &reducer3).await.0);

		// actions
		let a1 = actions(&storage, reducer1.log()).await;
		let a2 = actions(&storage, reducer2.log()).await;
		let a3 = actions(&storage, reducer3.log()).await;
		assert_eq!(a1, a2);
		assert_eq!(a1, a3);
	}

	async fn actions<S>(storage: &S, log: &Log) -> Vec<ReducerAction<CounterAction>>
	where
		S: ExtendedBlockStorage + Send + Sync + Clone + 'static,
	{
		log.stream(storage)
			.map(|entry| entry.unwrap().entry().payload)
			.then(move |cid| async move { storage.get(&cid).await })
			.map(|result| {
				BlockSerializer::new()
					.deserialize::<ReducerAction<CounterAction>>(&result.unwrap())
					.unwrap()
			})
			.collect()
			.await
	}

	async fn counter_state<S, R>(storage: &S, reducer: &Reducer<S, R>) -> Counter
	where
		S: ExtendedBlockStorage + Send + Sync + Clone + 'static,
		R: CoreResolver<S> + Send + Sync + 'static,
	{
		BlockSerializer::new()
			.deserialize(&storage.get(&reducer.state().unwrap()).await.unwrap())
			.unwrap()
	}

	#[tokio::test]
	async fn test_join_equal_heads() {
		// reducer
		let storage = MemoryBlockStorage::default();
		let identity = LocalIdentityResolver::default().private_identity("did:local:p1").unwrap();
		let log = Log::new(
			"test".as_bytes().to_vec(),
			IdentityEntryVerifier::new(IdentityResolverBox::new(LocalIdentityResolver::default())),
			Default::default(),
		);
		let runtime = RuntimePool::new(IdleRuntimePool::default());
		let native_core_resolver = SingleCoreResolver::new(Cid::default(), Core::native::<Counter>());
		let mut reducer = ReducerBuilder::new(native_core_resolver, log)
			.build(&storage, &runtime, MonotonicCoDate::default())
			.await
			.unwrap();

		// push
		reducer
			.push(&storage, &runtime, &identity, "test", &CounterAction::Increment(1))
			.await
			.unwrap();

		// add change handler
		struct Fail {}
		#[async_trait]
		impl ReducerChangedHandler<MemoryBlockStorage, SingleCoreResolver> for Fail {
			async fn on_state_changed(
				&mut self,
				_storage: &MemoryBlockStorage,
				_reducer: &Reducer<MemoryBlockStorage, SingleCoreResolver>,
				_context: ReducerChangeContext,
			) -> Result<(), anyhow::Error> {
				panic!("expected no state change when join same heads");
			}
		}
		reducer.add_change_handler(Box::new(Fail {}));

		// join
		assert!(reducer
			.join(&storage, &reducer.heads().clone(), &runtime)
			.await
			.unwrap()
			.is_none());
	}

	/// Compute State computes wrong result based on the ordering of the new heads when have multiple.
	/// This test aims to produce a case with both orders and test them to be correct.
	#[tokio::test]
	async fn test_compute_state_order_greater() {
		// // observed with this values:
		// //  not working
		// let a0 = Cid::from_str("bafyr4ib4umjz4wj4s7q5gfzrhgvinac5pykzu42uykrizucc236g2mxug4").unwrap();
		// let a1 = Cid::from_str("bafyr4igrex7fz64sc3yhokm3fqryyeox5s42lzyhjhtlfcwro7pbukeehq").unwrap();
		// //  working
		// let b0 = Cid::from_str("bafyr4ico4kfqhl6k3vdnrvwbnu5ozk373gk3u2ksftdymjevsrhill5yk4").unwrap();
		// let b1 = Cid::from_str("bafyr4iagbmtxiabpl3vgmjnppyse7tvasbeuma77axev5tomdpa642noqq").unwrap();
		// println!("{} > {} = {:?}", a0, a1, a0.cmp(&a1)); // Less
		// println!("{} > {} = {:?}", b0, b1, b0.cmp(&b1)); // Greater

		// reducer
		let storage = MemoryBlockStorage::default();
		let identity = LocalIdentityResolver::default().private_identity("did:local:p1").unwrap();
		let runtime = RuntimePool::new(IdleRuntimePool::default());
		let native_core_resolver = SingleCoreResolver::new(Cid::default(), Core::native::<Counter>());
		let co_date = MonotonicCoDate::default().boxed();

		// reducer1
		let mut reducer1 = ReducerBuilder::new(
			native_core_resolver.clone(),
			Log::new(
				"test".as_bytes().to_vec(),
				IdentityEntryVerifier::new(IdentityResolverBox::new(LocalIdentityResolver::default())),
				Default::default(),
			),
		)
		.build(&storage, &runtime, co_date.clone())
		.await
		.unwrap();
		reducer1
			.push(&storage, &runtime, &identity, "test", &CounterAction::Increment(1))
			.await
			.unwrap();

		// reducer2
		let mut reducer2 = ReducerBuilder::new(
			native_core_resolver.clone(),
			Log::new(
				"test".as_bytes().to_vec(),
				IdentityEntryVerifier::new(IdentityResolverBox::new(LocalIdentityResolver::default())),
				reducer1.heads().clone(),
			),
		)
		.with_snapshot(reducer1.state().unwrap(), reducer1.heads().clone())
		.build(&storage, &runtime, co_date.clone())
		.await
		.unwrap();
		assert_eq!(reducer1.state(), reducer2.state());
		assert_eq!(reducer1.heads(), reducer2.heads());

		// conflict
		reducer1
			.push(&storage, &runtime, &identity, "test", &CounterAction::Increment(1))
			.await
			.unwrap();
		reducer1
			.push(&storage, &runtime, &identity, "test", &CounterAction::Increment(4))
			.await
			.unwrap();
		reducer2
			.push(&storage, &runtime, &identity, "test", &CounterAction::Increment(1))
			.await
			.unwrap();
		reducer2
			.push(&storage, &runtime, &identity, "test", &CounterAction::Increment(2))
			.await
			.unwrap();
		let h1 = reducer1.heads().first().unwrap();
		let h2 = reducer2.heads().first().unwrap();
		println!("{} cmp {} = {:?}", h1, h2, h1.cmp(&h2));
		// bafyr4iff65doekq7e6jbbr6lfcaqw4yygr2xwnhcewk5n4x7656xgo3smq
		// cmp
		// bafyr4id7kpr5kduefd4j4s4lixevlrkbpbym2daylp7tztnqcdogg6ommq
		// =
		// Greater
		assert!(h1 > h2);

		// transfer state
		reducer1
			.insert_snapshot(&storage, reducer2.state().unwrap(), reducer2.heads().clone())
			.await
			.unwrap();
		reducer2
			.insert_snapshot(&storage, reducer1.state().unwrap(), reducer1.heads().clone())
			.await
			.unwrap();

		// join1
		reducer1.join(&storage, reducer2.heads(), &runtime).await.unwrap();
		assert_eq!(9, counter_state(&storage, &reducer1).await.0);

		// join2
		reducer2.join(&storage, reducer1.heads(), &runtime).await.unwrap();
		assert_eq!(9, counter_state(&storage, &reducer2).await.0);

		// test
		assert_eq!(reducer1.state(), reducer2.state());
		assert_eq!(reducer1.heads(), reducer2.heads());
	}

	/// Compute State computes wrong result based on the ordering of the new heads when have multiple.
	/// This test aims to produce a case with both orders and test them to be correct.
	#[tokio::test]
	async fn test_compute_state_order_less() {
		// // observed with this values:
		// //  not working
		// let a0 = Cid::from_str("bafyr4ib4umjz4wj4s7q5gfzrhgvinac5pykzu42uykrizucc236g2mxug4").unwrap();
		// let a1 = Cid::from_str("bafyr4igrex7fz64sc3yhokm3fqryyeox5s42lzyhjhtlfcwro7pbukeehq").unwrap();
		// //  working
		// let b0 = Cid::from_str("bafyr4ico4kfqhl6k3vdnrvwbnu5ozk373gk3u2ksftdymjevsrhill5yk4").unwrap();
		// let b1 = Cid::from_str("bafyr4iagbmtxiabpl3vgmjnppyse7tvasbeuma77axev5tomdpa642noqq").unwrap();
		// println!("{} > {} = {:?}", a0, a1, a0.cmp(&a1)); // Less
		// println!("{} > {} = {:?}", b0, b1, b0.cmp(&b1)); // Greater

		// reducer
		let storage = MemoryBlockStorage::default();
		let identity = LocalIdentityResolver::default().private_identity("did:local:p1").unwrap();
		let runtime = RuntimePool::new(IdleRuntimePool::default());
		let native_core_resolver = SingleCoreResolver::new(Cid::default(), Core::native::<Counter>());
		let co_date = MonotonicCoDate::default().boxed();

		// reducer1
		let mut reducer1 = ReducerBuilder::new(
			native_core_resolver.clone(),
			Log::new(
				"test".as_bytes().to_vec(),
				IdentityEntryVerifier::new(IdentityResolverBox::new(LocalIdentityResolver::default())),
				Default::default(),
			),
		)
		.build(&storage, &runtime, co_date.clone())
		.await
		.unwrap();
		reducer1
			.push(&storage, &runtime, &identity, "test", &CounterAction::Increment(1))
			.await
			.unwrap();

		// reducer2
		let mut reducer2 = ReducerBuilder::new(
			native_core_resolver.clone(),
			Log::new(
				"test".as_bytes().to_vec(),
				IdentityEntryVerifier::new(IdentityResolverBox::new(LocalIdentityResolver::default())),
				reducer1.heads().clone(),
			),
		)
		.with_snapshot(reducer1.state().unwrap(), reducer1.heads().clone())
		.build(&storage, &runtime, co_date.clone())
		.await
		.unwrap();
		assert_eq!(reducer1.state(), reducer2.state());
		assert_eq!(reducer1.heads(), reducer2.heads());

		// conflict
		reducer1
			.push(&storage, &runtime, &identity, "test", &CounterAction::Increment(1))
			.await
			.unwrap();
		reducer1
			.push(&storage, &runtime, &identity, "test", &CounterAction::Increment(2))
			.await
			.unwrap();
		reducer2
			.push(&storage, &runtime, &identity, "test", &CounterAction::Increment(1))
			.await
			.unwrap();
		reducer2
			.push(&storage, &runtime, &identity, "test", &CounterAction::Increment(2))
			.await
			.unwrap();
		let h1 = reducer1.heads().first().unwrap();
		let h2 = reducer2.heads().first().unwrap();
		println!("{} cmp {} = {:?}", h1, h2, h1.cmp(&h2));
		// bafyr4ib2txm6m2l4kbjghdpotl7tt54fzvwazsqs3lnoelwrbt4odqxzz4
		// cmp
		// bafyr4id7kpr5kduefd4j4s4lixevlrkbpbym2daylp7tztnqcdogg6ommq
		// =
		// Less
		assert!(h1 < h2);

		// transfer state
		reducer1
			.insert_snapshot(&storage, reducer2.state().unwrap(), reducer2.heads().clone())
			.await
			.unwrap();
		reducer2
			.insert_snapshot(&storage, reducer1.state().unwrap(), reducer1.heads().clone())
			.await
			.unwrap();

		// join1
		reducer1.join(&storage, reducer2.heads(), &runtime).await.unwrap();
		assert_eq!(7, counter_state(&storage, &reducer1).await.0);

		// join2
		reducer2.join(&storage, reducer1.heads(), &runtime).await.unwrap();
		assert_eq!(7, counter_state(&storage, &reducer2).await.0);

		// test
		assert_eq!(reducer1.state(), reducer2.state());
		assert_eq!(reducer1.heads(), reducer2.heads());
	}

	// // util: find a transaction which entry cid is less
	// let find = |state: Cid, heads: BTreeSet<Cid>| {
	// 	let storage = storage.clone();
	// 	let native_core_resolver = native_core_resolver.clone();
	// 	let runtime = runtime.clone();
	// 	let identity = identity.clone();
	// 	async move {
	// 		let mut reducer1 = ReducerBuilder::new(
	// 			native_core_resolver.clone(),
	// 			Log::new(
	// 				"test".as_bytes().to_vec(),
	// 				IdentityResolverBox::new(LocalIdentityResolver::default()),
	// 				heads.clone(),
	// 			),
	// 		)
	// 		.with_snapshot(state, heads.clone())
	// 		.build(&storage, &runtime, MonotonicCoDate::default())
	// 		.await
	// 		.unwrap();
	// 		reducer1
	// 			.push(&storage, &runtime, &identity, "test", &CounterAction::Increment(2))
	// 			.await
	// 			.unwrap();
	// 		let head1 = reducer1.heads().first().unwrap();
	// 		let mut count = 1;
	// 		loop {
	// 			let mut reducer2 = ReducerBuilder::new(
	// 				native_core_resolver.clone(),
	// 				Log::new(
	// 					"test".as_bytes().to_vec(),
	// 					IdentityResolverBox::new(LocalIdentityResolver::default()),
	// 					heads.clone(),
	// 				),
	// 			)
	// 			.with_snapshot(state, heads.clone())
	// 			.build(&storage, &runtime, MonotonicCoDate::default())
	// 			.await
	// 			.unwrap();
	// 			reducer2
	// 				.push(&storage, &runtime, &identity, "test", &CounterAction::Increment(count))
	// 				.await
	// 				.unwrap();
	// 			let head2 = reducer1.heads().first().unwrap();
	// 			if head1 < head2 {
	// 				return Result::<i64, anyhow::Error>::Ok(count);
	// 			}
	// 			count += 1;
	// 		}
	// 	}
	// };
	// let count = find(reducer1.state().unwrap(), reducer1.heads().clone()).await.unwrap();
	// println!("count: {}", count);

	/// Test `compute_stack` when we have no previous state to start calculation from.
	#[tokio::test]
	async fn test_compute_stack_without_previous_state() {
		// reducer
		let storage = MemoryBlockStorage::default();
		let co_date = MonotonicCoDate::default();
		let identity = LocalIdentityResolver::default().private_identity("did:local:p1").unwrap();
		let runtime = RuntimePool::new(IdleRuntimePool::default());
		let mut reducer1 = create_reducer(&storage, &runtime, &co_date, None).await;

		// push
		let action1 = reducer1
			.push(&storage, &runtime, &identity, "test", &CounterAction::Increment(1))
			.await
			.unwrap();

		// reducer2
		let mut reducer2 = create_reducer(&storage, &runtime, &co_date, Some(&reducer1)).await;
		assert_eq!((reducer2.state(), reducer2.heads()), (reducer1.state(), reducer1.heads()));

		// conflict
		let action2 = reducer1
			.push(&storage, &runtime, &identity, "test", &CounterAction::Increment(2))
			.await
			.unwrap();
		let action3 = reducer2
			.push(&storage, &runtime, &identity, "test", &CounterAction::Increment(3))
			.await
			.unwrap();

		// update reducer with reducer1 state
		reducer2.clear_snapshots();
		reducer2.log.join_heads(&storage, reducer1.log.heads()).await.unwrap();
		let (source_state, stack) = reducer2.compute_stack(&storage).await.unwrap();
		assert_eq!(source_state, None);
		assert_eq!(stack.len(), 3);
		assert_eq!(stack[0].cid(), action1.entry.cid());
		assert_eq!(stack[1].cid(), action3.entry.cid());
		assert_eq!(stack[2].cid(), action2.entry.cid());
	}

	/// Test `compute_stack` when we have a previous state to start calculation from.
	#[tokio::test]
	async fn test_compute_stack() {
		// reducer
		let storage = MemoryBlockStorage::default();
		let co_date = MonotonicCoDate::default();
		let identity = LocalIdentityResolver::default().private_identity("did:local:p1").unwrap();
		let runtime = RuntimePool::new(IdleRuntimePool::default());
		let mut reducer1 = create_reducer(&storage, &runtime, &co_date, None).await;

		// push
		let action1 = reducer1
			.push(&storage, &runtime, &identity, "test", &CounterAction::Increment(1))
			.await
			.unwrap();

		// reducer2
		let mut reducer2 = create_reducer(&storage, &runtime, &co_date, Some(&reducer1)).await;
		assert_eq!((reducer2.state(), reducer2.heads()), (reducer1.state(), reducer1.heads()));

		// conflict
		let action2 = reducer1
			.push(&storage, &runtime, &identity, "test", &CounterAction::Increment(2))
			.await
			.unwrap();
		let action3 = reducer2
			.push(&storage, &runtime, &identity, "test", &CounterAction::Increment(3))
			.await
			.unwrap();

		// update reducer with reducer1 state
		reducer2.log.join_heads(&storage, reducer1.log.heads()).await.unwrap();
		let (source_state, stack) = reducer2.compute_stack(&storage).await.unwrap();
		assert_eq!(source_state, Some(action1.state.unwrap()));
		assert_eq!(stack.len(), 2);
		assert_eq!(stack[0].cid(), action3.entry.cid());
		assert_eq!(stack[1].cid(), action2.entry.cid());
	}

	async fn create_reducer(
		storage: &MemoryBlockStorage,
		runtime: &RuntimePool,
		co_date: &MonotonicCoDate,
		from: Option<&Reducer<MemoryBlockStorage, SingleCoreResolver>>,
	) -> Reducer<MemoryBlockStorage, SingleCoreResolver> {
		let mut builder = ReducerBuilder::new(
			SingleCoreResolver::new(Cid::default(), Core::native::<Counter>()),
			Log::new_local("test".as_bytes().to_vec(), from.map(|log| log.heads().clone()).unwrap_or_default()),
		);
		if let Some(from) = from {
			builder = builder.with_snapshot(from.state().unwrap(), from.heads().clone());
		}
		builder.build(storage, runtime, co_date.clone()).await.unwrap()
	}
}
