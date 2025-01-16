use crate::CoreResolver;
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use cid::Cid;
use co_identity::PrivateIdentity;
use co_log::{EntryBlock, Log, LogError};
use co_primitives::ReducerAction;
use co_runtime::RuntimePool;
use co_storage::BlockStorage;
use futures::{pin_mut, stream, StreamExt, TryStreamExt};
use serde::Serialize;
use std::{
	collections::{BTreeSet, HashMap, VecDeque},
	time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::watch;
use tracing::instrument;

pub struct ReducerBuilder<S, R> {
	/// Storage.
	log: Log<S>,
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
}
impl<S, R> ReducerBuilder<S, R>
where
	S: BlockStorage + Send + Sync + Clone + 'static,
	R: CoreResolver<S> + Send + Sync + 'static,
{
	pub fn new(core_resolver: R, log: Log<S>) -> Self {
		Self {
			core_resolver,
			heads: Default::default(),
			snapshots: Default::default(),
			state: None,
			log,
			initialize: true,
		}
	}

	pub fn core_resolver_mut(&mut self) -> &mut R {
		&mut self.core_resolver
	}

	pub fn with_initialize(self, initialize: bool) -> Self {
		Self { initialize, ..self }
	}

	pub fn with_latest_state(self, state: Cid, heads: BTreeSet<Cid>) -> Self {
		Self { state: Some(state), heads, ..self }
	}

	pub fn with_snapshot(self, state: Cid, heads: BTreeSet<Cid>) -> Self {
		let mut snapshots = self.snapshots;
		snapshots.insert(heads, state);
		Self { snapshots, ..self }
	}

	pub async fn build(self, runtime: &RuntimePool) -> Result<Reducer<S, R>, anyhow::Error> {
		// validate heads
		if self.state.is_some() && self.log.heads() != &self.heads {
			return Err(anyhow!("Invalid heads. The log and state heads must be the same"));
		}

		// build
		let mut result = Reducer {
			core_resolver: self.core_resolver,
			heads: self.heads,
			snapshots: self.snapshots,
			state: self.state,
			log: self.log,
			change_handlers: Default::default(),
			watch: watch::channel(None),
		};
		if self.initialize {
			result.initialize(runtime).await?;
		}
		Ok(result)
	}
}

/// The reducers combines the log to a root state.
pub struct Reducer<S, R> {
	/// Storage.
	log: Log<S>,
	/// The core resolver which composes the state.
	core_resolver: R,
	/// Latest state.
	state: Option<Cid>,
	/// Latest heads.
	heads: BTreeSet<Cid>,
	/// Avilable historic snapshots (in chronologic order?).
	snapshots: HashMap<BTreeSet<Cid>, Cid>,
	/// Change handlers.
	change_handlers: Vec<Box<dyn ReducerChangedHandler<S, R> + Send + Sync>>,
	/// State/Heads watcher.
	watch: (watch::Sender<Option<(Cid, BTreeSet<Cid>)>>, watch::Receiver<Option<(Cid, BTreeSet<Cid>)>>),
}
impl<S, R> Reducer<S, R>
where
	S: BlockStorage + Send + Sync + Clone + 'static,
	R: CoreResolver<S> + Send + Sync + 'static,
{
	/// Initialize this reducer by computing current state if one.
	#[instrument(skip(self, runtime))]
	pub async fn initialize(&mut self, runtime: &RuntimePool) -> Result<(), anyhow::Error> {
		tracing::trace!(?self.snapshots, "reducer-initialize");
		let context = ReducerChangeContext { cause: ReducerChangeCause::Initialize };

		// if we have snapshots but no state/heads join all heads from snapshots
		// find latest state if we have snapshots but no latest selection
		if self.state.is_none() && self.heads.is_empty() && !self.snapshots.is_empty() {
			for (heads, _) in self.snapshots.iter() {
				// join heads
				self.log.join_heads(heads.iter()).await?;

				// try to find state for latest heads
				// do this every iteration so we end up with the latest known state
				for (snapshot_heads, snapshot_state) in &self.snapshots {
					if snapshot_heads == self.log.heads() {
						self.state = Some(*snapshot_state);
						self.heads = snapshot_heads.clone();
					}
				}
			}
			tracing::trace!(state = ?self.state, heads = ?self.heads, log_heads = ?self.log.heads(), "reducer-snapshots");
		}

		// if log heads are different from reducer heads
		if &self.heads != self.log.heads() {
			// compute the state
			let (state, heads) = self.compute_state(runtime, &context).await?;
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
		self.on_state_changed(&context).await?;

		// log
		tracing::trace!(?self.state, ?self.heads, "reducer-initialized");

		// if we have state and heads we are fine
		Ok(())
	}

	pub fn into_log(self) -> Log<S> {
		self.log
	}

	pub fn is_empty(&self) -> bool {
		self.heads.is_empty() && self.state.is_none()
	}

	/// Get state observable.
	pub fn watch(&self) -> watch::Receiver<Option<(Cid, BTreeSet<Cid>)>> {
		self.watch.1.clone()
	}

	/// Add change handler which will be called when state changed.
	/// All change handlers will be called in parallel.
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
	pub fn log(&self) -> &Log<S> {
		&self.log
	}

	/// The log.
	pub fn log_mut(&mut self) -> &mut Log<S> {
		&mut self.log
	}

	/// Insert previous snapshots (trusted) of the same log from which we can continue.
	pub fn insert_snapshot(&mut self, state: Cid, heads: BTreeSet<Cid>) {
		self.snapshots.insert(heads, state);
	}

	/// Find the state matching the specified heads, if one.
	fn find_state(&self, heads: &BTreeSet<Cid>) -> Option<Cid> {
		if self.heads.eq(heads) {
			return self.state;
		}
		self.snapshots.get(heads).cloned()
	}

	/// Push an event.
	pub async fn push<T, I>(
		&mut self,
		runtime: &RuntimePool,
		identity: &I,
		core: &str,
		item: &T,
	) -> Result<Option<Cid>, anyhow::Error>
	where
		T: Serialize + Send + Sync,
		I: PrivateIdentity + Send + Sync,
	{
		let action = ReducerAction {
			core: core.to_owned(),
			payload: item,
			from: identity.identity().to_owned(),
			time: SystemTime::now().duration_since(UNIX_EPOCH).expect("Valid time").as_millis(),
		};
		self.push_action(runtime, identity, &action).await
	}

	/// Push an event.
	pub async fn push_action<T, I>(
		&mut self,
		runtime: &RuntimePool,
		identity: &I,
		action: &ReducerAction<T>,
	) -> Result<Option<Cid>, anyhow::Error>
	where
		T: Serialize + Send + Sync,
		I: PrivateIdentity + Send + Sync,
	{
		// validate
		if identity.identity() != action.from {
			return Err(anyhow!("Invalid argument: identity"));
		}

		// apply to log
		let (_, action) = self.log.push_event(identity, &action).await?;

		// // debug
		// let block = self.log.storage().get(entry.as_ref()).await.unwrap();
		// let ipld: Ipld = IpldCodec::DagCbor.decode(block.data()).unwrap();
		// println!("entry = {:?}", ipld);

		// apply to state
		let context = ReducerChangeContext { cause: ReducerChangeCause::Push };
		let state = self
			.core_resolver
			.execute(self.log.storage(), runtime, &context, &self.state, action.cid())
			.await?;

		// snapshot
		if self.state.is_some() {
			self.insert_snapshot(self.state.unwrap(), self.heads.clone());
		}

		// update
		self.state = state;
		self.heads = self.log.heads_iter().cloned().collect();

		// notify
		self.on_state_changed(&context).await?;

		// result
		Ok(state)
	}
	/// Join heads (from other log).
	/// This is used to join logs from other peers.
	/// Returns true if state has changed.
	pub async fn join(&mut self, heads: &BTreeSet<Cid>, runtime: &RuntimePool) -> Result<bool, LogError> {
		let mut result = false;
		if self.log().heads() != heads
			&& (self.log_mut().join_heads(heads.iter()).await? || &self.heads != self.log.heads())
		{
			// sync state
			let context = ReducerChangeContext { cause: ReducerChangeCause::Log };
			let (next_state, next_heads) = self.compute_state(runtime, &context).await?;
			result = next_state != self.state;
			if next_state != self.state || self.heads != next_heads {
				// apply
				self.state = next_state;
				self.heads = next_heads;

				// notify
				self.on_state_changed(&context).await?;
			}
		}
		Ok(result)
	}

	/// Notify subscribers about change.
	async fn on_state_changed(&mut self, context: &ReducerChangeContext) -> Result<(), LogError> {
		// handlers
		// note:
		//  we use try_for_each_concurrent which spawns each item on a new task this is required
		//  to prevent deadlocks because the on_state_changed handler is called within an outer RwLock.
		let mut change_handlers = Vec::new();
		change_handlers.append(&mut self.change_handlers);
		{
			let reducer: &Self = self;
			let context: &ReducerChangeContext = &context;
			stream::iter(change_handlers.iter_mut())
				.map(Ok)
				.try_for_each_concurrent(5, |handler| async move {
					handler
						.on_state_changed(reducer, context.clone())
						.await
						.with_context(|| "running ReducerChangeHandler".to_string())
				})
				.await?;
		}
		self.change_handlers.append(&mut change_handlers);

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
	#[instrument(err, skip(self, runtime))]
	async fn compute_state(
		&self,
		runtime: &RuntimePool,
		context: &ReducerChangeContext,
	) -> Result<(Option<Cid>, BTreeSet<Cid>), anyhow::Error> {
		// compute stack
		let (mut state, stack) = self.compute_stack().await?;

		// apply stack
		for entry in stack {
			state = self
				.core_resolver
				.execute(self.log.storage(), runtime, context, &state, &entry.entry().payload)
				.await?;
		}

		// result
		Ok((state, self.log.heads().clone()))
	}

	/// Compute stack to apply to an state.
	/// The computed start position is self.heads.
	/// The computed end position is self.log.heads.
	/// Algorithm: We search for the lowest known ancestors of the heads while walking the log backwards.
	#[instrument(skip(self))]
	async fn compute_stack(&self) -> Result<(Option<Cid>, VecDeque<EntryBlock<S::StoreParams>>), anyhow::Error> {
		let heads: BTreeSet<Cid> = self.log.heads().clone();
		let mut state = self.state;
		let mut stack = VecDeque::new();

		// is latest state?
		if self.heads != heads {
			// find latest usable historic state
			let entries = self.log.stream();
			pin_mut!(entries);
			let mut missing_heads = heads.clone();
			let mut state_events: Option<BTreeSet<Cid>> = None;
			while let Some(entry) = entries.next().await {
				let entry = entry?;

				// when we found a state we continue to go back until we see all of its entry heads (entry.next)
				if let Some(state_events) = &mut state_events {
					// remove seen elements
					if state_events.remove(entry.cid()) {
						// if we have seen all entries which generated the found state the stack is complete
						// and ready to reapply
						if state_events.is_empty() {
							break;
						}
						continue;
					}
				} else {
					// remove all seen entries from missing
					// when we have seen all heads we can search for an known state
					missing_heads.remove(entry.cid());
					if missing_heads.is_empty() {
						// does this entry reference a state we know?
						// note: this will never match if we have no previous states
						//  and a new state will be recomputed from scratch
						if let Some(entry_state) = self.find_state(&entry.entry().next) {
							state = Some(entry_state);
							state_events = Some(entry.entry().next.clone());
						}
					}
				}

				// put on stack to reapply
				stack.push_front(entry);
			}
		}

		// result
		Ok((state, stack))
	}
}

/// Reducer change handler.
/// Will be executed everytime the state in the reducer changes, including on initialize.
#[async_trait]
pub trait ReducerChangedHandler<S, R> {
	async fn on_state_changed(
		&mut self,
		reducer: &Reducer<S, R>,
		context: ReducerChangeContext,
	) -> Result<(), anyhow::Error>;
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
		application::reducer::ReducerBuilder, CoreResolver, ReducerChangeContext, ReducerChangedHandler,
		SingleCoreResolver,
	};
	use async_trait::async_trait;
	use co_identity::{IdentityResolverBox, LocalIdentityResolver};
	use co_log::Log;
	use co_primitives::{BlockSerializer, ReducerAction};
	use co_runtime::{Core, IdleRuntimePool, RuntimePool};
	use co_storage::{unixfs_add_file, BlockStorage, MemoryBlockStorage};
	use example_counter::{Counter, CounterAction};
	use futures::StreamExt;
	use tokio::process::Command;

	#[tokio::test]
	async fn smoke() {
		// store
		let storage = MemoryBlockStorage::new();

		// wasm
		Command::new("cargo")
			.current_dir("../examples/counter")
			.args(["build", "--target=wasm32-unknown-unknown", "--release"])
			.output()
			.await
			.unwrap();
		let wasm = unixfs_add_file(&storage, "../target/wasm32-unknown-unknown/release/example_counter.wasm")
			.await
			.unwrap();

		// logs
		let identity1 = LocalIdentityResolver::default().private_identity("did:local:p1").unwrap();
		let identity2 = LocalIdentityResolver::default().private_identity("did:local:p2").unwrap();
		let identity3 = LocalIdentityResolver::default().private_identity("did:local:p3").unwrap();
		let log1 = Log::new(
			"test".as_bytes().to_vec(),
			IdentityResolverBox::new(LocalIdentityResolver::default()),
			storage.clone(),
			Default::default(),
		);
		let log2 = Log::new(
			"test".as_bytes().to_vec(),
			IdentityResolverBox::new(LocalIdentityResolver::default()),
			storage.clone(),
			Default::default(),
		);
		let log3 = Log::new(
			"test".as_bytes().to_vec(),
			IdentityResolverBox::new(LocalIdentityResolver::default()),
			storage.clone(),
			Default::default(),
		);

		// pool
		let runtime = RuntimePool::new(IdleRuntimePool::default());
		let core_resolver = SingleCoreResolver::new(wasm.into());
		let native_core_resolver = SingleCoreResolver::new(Core::native::<Counter>());

		// reducer
		let mut reducer1 = ReducerBuilder::new(native_core_resolver, log1).build(&runtime).await.unwrap();
		let mut reducer2 = ReducerBuilder::new(core_resolver.clone(), log2).build(&runtime).await.unwrap();
		let mut reducer3 = ReducerBuilder::new(core_resolver.clone(), log3).build(&runtime).await.unwrap();

		// 1-6
		reducer1
			.push(&runtime, &identity1, "test", &CounterAction::Increment(1))
			.await
			.unwrap();
		reducer1
			.push(&runtime, &identity1, "test", &CounterAction::Increment(2))
			.await
			.unwrap();
		reducer1
			.push(&runtime, &identity1, "test", &CounterAction::Increment(3))
			.await
			.unwrap();
		reducer1
			.push(&runtime, &identity1, "test", &CounterAction::Increment(4))
			.await
			.unwrap();
		reducer1
			.push(&runtime, &identity1, "test", &CounterAction::Increment(5))
			.await
			.unwrap();
		reducer1
			.push(&runtime, &identity1, "test", &CounterAction::Increment(6))
			.await
			.unwrap();
		reducer2.join(reducer1.heads(), &runtime).await.unwrap();
		reducer3.join(reducer1.heads(), &runtime).await.unwrap();
		assert_eq!(21, counter_state(&storage, &reducer1).await.0); // 1 + 2 + 3 + 4 + 5 + 6
		assert_eq!(21, counter_state(&storage, &reducer2).await.0);
		assert_eq!(21, counter_state(&storage, &reducer3).await.0);

		// 7
		reducer2
			.push(&runtime, &identity2, "test", &CounterAction::Increment(7))
			.await
			.unwrap();
		reducer3.join(reducer2.heads(), &runtime).await.unwrap();
		reducer1.join(reducer3.heads(), &runtime).await.unwrap();
		assert_eq!(28, counter_state(&storage, &reducer1).await.0);
		assert_eq!(28, counter_state(&storage, &reducer2).await.0);
		assert_eq!(28, counter_state(&storage, &reducer3).await.0);

		// 8
		reducer3
			.push(&runtime, &identity3, "test", &CounterAction::Increment(8))
			.await
			.unwrap();
		reducer2.join(reducer3.heads(), &runtime).await.unwrap();
		reducer1.join(reducer2.heads(), &runtime).await.unwrap();
		assert_eq!(36, counter_state(&storage, &reducer1).await.0);
		assert_eq!(36, counter_state(&storage, &reducer2).await.0);
		assert_eq!(36, counter_state(&storage, &reducer3).await.0);

		// 9
		reducer3
			.push(&runtime, &identity3, "test", &CounterAction::Increment(9))
			.await
			.unwrap();
		reducer2.join(reducer3.heads(), &runtime).await.unwrap();
		reducer1.join(reducer2.heads(), &runtime).await.unwrap();
		assert_eq!(45, counter_state(&storage, &reducer1).await.0);
		assert_eq!(45, counter_state(&storage, &reducer2).await.0);
		assert_eq!(45, counter_state(&storage, &reducer3).await.0);

		// A
		reducer1
			.push(&runtime, &identity1, "test", &CounterAction::Increment(10))
			.await
			.unwrap();
		reducer2.join(reducer1.heads(), &runtime).await.unwrap();
		reducer3.join(reducer2.heads(), &runtime).await.unwrap();
		assert_eq!(55, counter_state(&storage, &reducer1).await.0);
		assert_eq!(55, counter_state(&storage, &reducer2).await.0);
		assert_eq!(55, counter_state(&storage, &reducer3).await.0);

		// B
		reducer1
			.push(&runtime, &identity1, "test", &CounterAction::Set(11))
			.await
			.unwrap();
		reducer2.join(reducer1.heads(), &runtime).await.unwrap();
		assert_eq!(11, counter_state(&storage, &reducer1).await.0);
		assert_eq!(11, counter_state(&storage, &reducer2).await.0);
		assert_eq!(55, counter_state(&storage, &reducer3).await.0);

		// C
		reducer1
			.push(&runtime, &identity1, "test", &CounterAction::Increment(12))
			.await
			.unwrap();
		reducer2
			.push(&runtime, &identity2, "test", &CounterAction::Increment(12))
			.await
			.unwrap();
		reducer2.join(reducer1.heads(), &runtime).await.unwrap();
		reducer1.join(reducer2.heads(), &runtime).await.unwrap();
		assert_eq!(35, counter_state(&storage, &reducer1).await.0);
		assert_eq!(35, counter_state(&storage, &reducer2).await.0);
		assert_eq!(55, counter_state(&storage, &reducer3).await.0);

		// D
		reducer1
			.push(&runtime, &identity1, "test", &CounterAction::Increment(13))
			.await
			.unwrap();
		reducer2.join(reducer1.heads(), &runtime).await.unwrap();
		assert_eq!(48, counter_state(&storage, &reducer1).await.0);
		assert_eq!(48, counter_state(&storage, &reducer2).await.0);
		assert_eq!(55, counter_state(&storage, &reducer3).await.0);

		// E
		reducer2
			.push(&runtime, &identity2, "test", &CounterAction::Increment(14))
			.await
			.unwrap();
		reducer1.join(reducer2.heads(), &runtime).await.unwrap();
		assert_eq!(62, counter_state(&storage, &reducer1).await.0);
		assert_eq!(62, counter_state(&storage, &reducer2).await.0);
		assert_eq!(55, counter_state(&storage, &reducer3).await.0);

		// B*
		reducer3
			.push(&runtime, &identity3, "test", &CounterAction::Increment(11))
			.await
			.unwrap();
		reducer3.join(reducer1.heads(), &runtime).await.unwrap();
		reducer2.join(reducer3.heads(), &runtime).await.unwrap();
		reducer1.join(reducer2.heads(), &runtime).await.unwrap();
		assert_eq!(73, counter_state(&storage, &reducer1).await.0);
		assert_eq!(73, counter_state(&storage, &reducer2).await.0);
		assert_eq!(73, counter_state(&storage, &reducer3).await.0);

		// actions
		let a1 = actions(reducer1.log()).await;
		let a2 = actions(reducer2.log()).await;
		let a3 = actions(reducer3.log()).await;
		assert_eq!(a1, a2);
		assert_eq!(a1, a3);
	}

	async fn actions<S>(log: &Log<S>) -> Vec<ReducerAction<CounterAction>>
	where
		S: BlockStorage + Send + Sync + Clone + 'static,
	{
		let storage_ref = log.storage();
		log.stream()
			.map(|entry| entry.unwrap().entry().payload)
			.then(move |cid| async move { storage_ref.clone().get(&cid).await })
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
		S: BlockStorage + Send + Sync + Clone + 'static,
		R: CoreResolver<S> + Send + Sync + 'static,
	{
		BlockSerializer::new()
			.deserialize(&storage.get(&reducer.state().unwrap()).await.unwrap())
			.unwrap()
	}

	#[tokio::test]
	async fn test_join_equal_heads() {
		// reducer
		let storage = MemoryBlockStorage::new();
		let identity = LocalIdentityResolver::default().private_identity("did:local:p1").unwrap();
		let log = Log::new(
			"test".as_bytes().to_vec(),
			IdentityResolverBox::new(LocalIdentityResolver::default()),
			storage.clone(),
			Default::default(),
		);
		let runtime = RuntimePool::new(IdleRuntimePool::default());
		let native_core_resolver = SingleCoreResolver::new(Core::native::<Counter>());
		let mut reducer = ReducerBuilder::new(native_core_resolver, log).build(&runtime).await.unwrap();

		// push
		reducer
			.push(&runtime, &identity, "test", &CounterAction::Increment(1))
			.await
			.unwrap();

		// add change handler
		struct Fail {}
		#[async_trait]
		impl ReducerChangedHandler<MemoryBlockStorage, SingleCoreResolver> for Fail {
			async fn on_state_changed(
				&mut self,
				_reducer: &Reducer<MemoryBlockStorage, SingleCoreResolver>,
				_context: ReducerChangeContext,
			) -> Result<(), anyhow::Error> {
				panic!("expected no state change when join same heads");
			}
		}
		reducer.add_change_handler(Box::new(Fail {}));

		// join
		assert!(!reducer.join(&reducer.heads().clone(), &runtime).await.unwrap());
	}
}
