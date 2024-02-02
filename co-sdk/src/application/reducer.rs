use anyhow::anyhow;
use co_log::Log;
use co_primitives::{Linkable, ReducerAction};
use co_runtime::{Core, RuntimeContext, RuntimePool};
use co_storage::BlockStorage;
use libipld::Cid;
use serde::Serialize;
use std::{
	collections::{BTreeSet, HashMap},
	time::{SystemTime, UNIX_EPOCH},
};

pub struct ReducerBuilder<S> {
	/// Storage.
	log: Log<S>,
	/// The (root) core which composes the state.
	core: Core,
	/// Latest state.
	state: Option<Cid>,
	/// Latests heads.
	heads: BTreeSet<Cid>,
	/// Avilable historic snapshots.
	snapshots: HashMap<BTreeSet<Cid>, Cid>,
}
impl<S> ReducerBuilder<S>
where
	S: BlockStorage + Send + Sync + Clone + 'static,
{
	pub fn new(core: Core, log: Log<S>) -> Self {
		Self { core, heads: Default::default(), snapshots: Default::default(), state: None, log }
	}

	pub fn with_latest_state(self, state: Cid, heads: BTreeSet<Cid>) -> Self {
		Self { state: Some(state), heads, ..self }
	}

	pub fn with_snapshot(self, state: Cid, heads: BTreeSet<Cid>) -> Self {
		let mut snapshots = self.snapshots;
		snapshots.insert(heads, state);
		Self { snapshots, ..self }
	}

	pub async fn build(self) -> Result<Reducer<S>, anyhow::Error> {
		// validate heads
		if self.state.is_some() && !self.log.heads_iter().eq(self.heads.iter()) {
			return Err(anyhow!("Invalid heads. The log and state heads must be the same"));
		}

		// build
		let mut result =
			Reducer { core: self.core, heads: self.heads, snapshots: self.snapshots, state: self.state, log: self.log };
		result.initialize().await?;
		Ok(result)
	}
}

pub struct Reducer<S> {
	/// Storage.
	log: Log<S>,
	/// The (root) core which composes the state.
	core: Core,
	/// Latest state.
	state: Option<Cid>,
	/// Latests heads.
	heads: BTreeSet<Cid>,
	/// Avilable historic snapshots in chronologic order.
	snapshots: HashMap<BTreeSet<Cid>, Cid>,
}
impl<S> Reducer<S>
where
	S: BlockStorage + Send + Sync + Clone + 'static,
{
	/// Initialize this reducer by computing current state if one.
	pub async fn initialize(&mut self) -> Result<(), anyhow::Error> {
		if self.state.is_none() {}
		Ok(())
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

	/// Find start position to continue with heads.
	/// If None is retuned the state needs to be recreated from log root.
	async fn find_start(heads: BTreeSet<Cid>) -> Option<(Cid, BTreeSet<Cid>)> {
		todo!()
	}

	/// Create the initial state for the core.
	pub async fn create_initial_state(core: Cid, storage: S) -> Result<Cid, anyhow::Error> {
		todo!()
	}

	/// Push an event.
	pub async fn push<T: Serialize + 'static>(
		&mut self,
		runtime: &RuntimePool,
		co: &str,
		item: &T,
	) -> Result<(), anyhow::Error> {
		// apply to log
		let action = ReducerAction {
			core: co.to_owned(),
			payload: item,
			from: self.log.identity().identity().to_owned(),
			time: SystemTime::now().duration_since(UNIX_EPOCH).expect("Valid time").as_millis(),
		};
		let (_, action) = self.log.push_event(&action).await?;

		// // debug
		// let block = self.log.storage().get(entry.as_ref()).await.unwrap();
		// let ipld: Ipld = IpldCodec::DagCbor.decode(block.data()).unwrap();
		// println!("entry = {:?}", ipld);

		// apply to state
		let state = runtime
			.execute(self.log.storage(), &self.core, RuntimeContext { state: self.state, event: action.into() })
			.await?;

		// snapshot
		if self.state.is_some() {
			self.insert_snapshot(self.state.unwrap(), self.heads.clone());
		}

		// update
		self.state = state;
		self.heads = self.log.heads_iter().cloned().collect();

		// result
		Ok(())
	}

	/// Join heads.
	/// This is used to join logs from other peers.
	pub async fn join(heads: BTreeSet<Cid>) {}
}

#[cfg(test)]
mod tests {
	use super::Reducer;
	use crate::application::reducer::ReducerBuilder;
	use co_log::{LocalIdentityResolver, Log};
	use co_primitives::BlockSerializer;
	use co_runtime::{Core, IdleRuntimePool, RuntimePool};
	use co_storage::{store_file, BlockStorage, MemoryBlockStorage};
	use example_counter::{Counter, CounterAction};
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
		let wasm = store_file(&storage, "../target/wasm32-unknown-unknown/release/example_counter.wasm")
			.await
			.unwrap();

		// logs
		let log1 = Log::new(
			"test".as_bytes().to_vec(),
			LocalIdentityResolver::default().private_identity("did:local:p1").unwrap(),
			Box::new(LocalIdentityResolver::default()),
			storage.clone(),
			Vec::new(),
		);
		let log2 = Log::new(
			"test".as_bytes().to_vec(),
			LocalIdentityResolver::default().private_identity("did:local:p2").unwrap(),
			Box::new(LocalIdentityResolver::default()),
			storage.clone(),
			Vec::new(),
		);
		let log3 = Log::new(
			"test".as_bytes().to_vec(),
			LocalIdentityResolver::default().private_identity("did:local:p3").unwrap(),
			Box::new(LocalIdentityResolver::default()),
			storage.clone(),
			Vec::new(),
		);

		// pool
		let runtime = RuntimePool::new(IdleRuntimePool::default());

		// reducer
		let mut reducer1 = ReducerBuilder::new(Core::native::<Counter>(), log1).build().await.unwrap();
		let mut reducer2 = ReducerBuilder::new(wasm.into(), log2).build().await.unwrap();
		let mut reducer3 = ReducerBuilder::new(wasm.into(), log3).build().await.unwrap();

		// 1-6
		reducer1.push(&runtime, "test", &CounterAction::Increment(1)).await.unwrap();
		reducer1.push(&runtime, "test", &CounterAction::Increment(2)).await.unwrap();
		reducer1.push(&runtime, "test", &CounterAction::Increment(3)).await.unwrap();
		reducer1.push(&runtime, "test", &CounterAction::Increment(4)).await.unwrap();
		reducer1.push(&runtime, "test", &CounterAction::Increment(5)).await.unwrap();
		reducer1.push(&runtime, "test", &CounterAction::Increment(6)).await.unwrap();
		reducer2.log_mut().join(&reducer1.log()).await.unwrap();
		reducer3.log_mut().join(&reducer1.log()).await.unwrap();
		assert_eq!(21, counter_state(&storage, &reducer1).await.0); // 1 + 2 + 3 + 4 + 5 + 6
		assert_eq!(21, counter_state(&storage, &reducer2).await.0);
		assert_eq!(21, counter_state(&storage, &reducer3).await.0);

		// 7
		reducer2.push(&runtime, "test", &CounterAction::Increment(7)).await.unwrap();
		reducer3.log_mut().join(&reducer2.log()).await.unwrap();
		reducer1.log_mut().join(&reducer3.log()).await.unwrap();
		assert_eq!(28, counter_state(&storage, &reducer1).await.0);
		assert_eq!(28, counter_state(&storage, &reducer2).await.0);
		assert_eq!(28, counter_state(&storage, &reducer3).await.0);

		// 8
		reducer3.push(&runtime, "test", &CounterAction::Increment(8)).await.unwrap();
		reducer2.log_mut().join(&reducer3.log()).await.unwrap();
		reducer1.log_mut().join(&reducer2.log()).await.unwrap();
		assert_eq!(36, counter_state(&storage, &reducer1).await.0);
		assert_eq!(36, counter_state(&storage, &reducer2).await.0);
		assert_eq!(36, counter_state(&storage, &reducer3).await.0);

		// 9
		reducer3.push(&runtime, "test", &CounterAction::Increment(9)).await.unwrap();
		reducer2.log_mut().join(&reducer3.log()).await.unwrap();
		reducer1.log_mut().join(&reducer2.log()).await.unwrap();
		assert_eq!(45, counter_state(&storage, &reducer1).await.0);
		assert_eq!(45, counter_state(&storage, &reducer2).await.0);
		assert_eq!(45, counter_state(&storage, &reducer3).await.0);

		// A, B
		reducer1.push(&runtime, "test", &CounterAction::Increment(10)).await.unwrap();
		reducer1.push(&runtime, "test", &CounterAction::Set(11)).await.unwrap();
		reducer2.log_mut().join(&reducer1.log()).await.unwrap();
		assert_eq!(11, counter_state(&storage, &reducer1).await.0);
		assert_eq!(11, counter_state(&storage, &reducer2).await.0);
		assert_eq!(45, counter_state(&storage, &reducer3).await.0);

		// C
		reducer1.push(&runtime, "test", &CounterAction::Increment(12)).await.unwrap();
		reducer2.push(&runtime, "test", &CounterAction::Increment(12)).await.unwrap();
		reducer2.log_mut().join(&reducer1.log()).await.unwrap();
		reducer1.log_mut().join(&reducer2.log()).await.unwrap();
		assert_eq!(35, counter_state(&storage, &reducer1).await.0);
		assert_eq!(35, counter_state(&storage, &reducer2).await.0);
		assert_eq!(45, counter_state(&storage, &reducer3).await.0);

		// D
		reducer1.push(&runtime, "test", &CounterAction::Increment(13)).await.unwrap();
		reducer2.log_mut().join(&reducer1.log()).await.unwrap();
		assert_eq!(48, counter_state(&storage, &reducer1).await.0);
		assert_eq!(48, counter_state(&storage, &reducer2).await.0);
		assert_eq!(45, counter_state(&storage, &reducer3).await.0);

		// E
		reducer2.push(&runtime, "test", &CounterAction::Increment(14)).await.unwrap();
		reducer1.log_mut().join(&reducer2.log()).await.unwrap();
		assert_eq!(62, counter_state(&storage, &reducer1).await.0);
		assert_eq!(62, counter_state(&storage, &reducer2).await.0);
		assert_eq!(45, counter_state(&storage, &reducer3).await.0);

		// B*
		reducer3.push(&runtime, "test", &CounterAction::Increment(11)).await.unwrap();
		reducer2.log_mut().join(&reducer3.log()).await.unwrap();
		reducer1.log_mut().join(&reducer2.log()).await.unwrap();
		assert_eq!(73, counter_state(&storage, &reducer1).await.0);
		assert_eq!(73, counter_state(&storage, &reducer2).await.0);
		assert_eq!(73, counter_state(&storage, &reducer3).await.0);
	}

	async fn counter_state<S>(storage: &S, reducer: &Reducer<S>) -> Counter
	where
		S: BlockStorage + Send + Sync + Clone + 'static,
	{
		BlockSerializer::new()
			.deserialize(&storage.get(&reducer.state().unwrap()).await.unwrap())
			.unwrap()
	}
}
