use crate::{
	application::memory::create_memory_reducer, reducer::core_resolver::dynamic::DynamicCoreResolver,
	types::co_dispatch::CoDispatch, CoReducerState, DynamicCoDate, Reducer, Runtime, Storage,
};
use anyhow::anyhow;
use async_trait::async_trait;
use cid::Cid;
use co_actor::TaskSpawner;
use co_identity::PrivateIdentityBox;
use co_primitives::{CoId, ReducerAction};
use co_storage::{
	BlockStorageContentMapping, BlockStorageExt, ExtendedBlockStorage, OverlayBlockStorage, StoreParamsBlockStorage,
};
use serde::Serialize;
use std::{collections::BTreeSet, marker::PhantomData, mem::take};

pub struct MemoryDispatch<S, A>
where
	S: ExtendedBlockStorage + BlockStorageContentMapping + Clone + 'static,
{
	_action: PhantomData<A>,

	runtime: Runtime,
	date: DynamicCoDate,

	id: CoId,
	core: String,
	identity: PrivateIdentityBox,

	reducer: Reducer<OverlayBlockStorage<S>, DynamicCoreResolver<OverlayBlockStorage<S>>>,
	reducer_storage: OverlayBlockStorage<S>,

	new_roots: Vec<CoReducerState>,
}
impl<S, A> MemoryDispatch<S, A>
where
	A: Serialize + Clone + Send + Sync + 'static,
	S: ExtendedBlockStorage + BlockStorageContentMapping + Clone + 'static,
{
	/// Create memory dispatcher.
	///
	/// # Args
	/// - `id` - The id of the CO.
	/// - `reducer_state` - The state of the CO.
	/// - `storage` - The base storage of the CO.
	/// - `identity` - The identity to push actions.
	/// - `core` - The core name to push to.
	pub async fn new(
		storage: Storage,
		runtime: Runtime,
		date: DynamicCoDate,
		tasks: TaskSpawner,
		id: CoId,
		reducer_state: CoReducerState,
		reducer_storage: &S,
		identity: PrivateIdentityBox,
		core: impl Into<String>,
	) -> Result<Self, anyhow::Error>
	where
		S: ExtendedBlockStorage + Clone + 'static,
	{
		let tmp = StoreParamsBlockStorage::new(storage.tmp_storage(), false);
		let reducer_storage = OverlayBlockStorage::new(tasks, reducer_storage.clone(), tmp, None, true, false);
		let reducer =
			create_memory_reducer(runtime.runtime(), date.clone(), &id, &reducer_storage, reducer_state).await?;
		Ok(Self {
			id,
			reducer,
			runtime,
			core: core.into(),
			identity,
			reducer_storage,
			new_roots: Default::default(),
			_action: PhantomData,
			date,
		})
	}

	/// Reset this instance to a specific state.
	/// Note: This will not clear the storage.
	pub async fn reset(&mut self, reducer_state: CoReducerState) -> Result<(), anyhow::Error> {
		self.reducer = create_memory_reducer(
			self.runtime.runtime(),
			self.date.clone(),
			&self.id,
			&self.reducer_storage,
			reducer_state,
		)
		.await?;
		self.new_roots.clear();
		Ok(())
	}

	/// Push action with an precomputed state.
	/// Note: This is dangerous if `unsafe_skip_verify` is used and the caller is responsible to know that
	///  `action + current state = state`.
	pub async fn push_with_state(
		&mut self,
		action: &ReducerAction<A>,
		state: Cid,
		unsafe_skip_verify: bool,
	) -> Result<(), anyhow::Error> {
		// verify
		if action.core != self.core {
			return Err(anyhow!("Invalid core name"));
		}

		// push
		if unsafe_skip_verify {
			let action_reference = self.reducer_storage.set_value(action).await?;
			let head = self
				.reducer
				.log_mut()
				.push(&self.reducer_storage, &self.identity, *action_reference.cid())
				.await?;
			let heads: BTreeSet<Cid> = [*head.cid()].into_iter().collect();
			self.reducer.insert_snapshot(state, heads.clone());
			self.reducer.join(&self.reducer_storage, &heads, self.runtime.runtime()).await?;
		} else {
			let verify_state = self
				.reducer
				.push_action(&self.reducer_storage, self.runtime.runtime(), &self.identity, action)
				.await?;
			if verify_state != Some(state) {
				return Err(anyhow!("Verify action failed"));
			}
		}

		// record
		self.new_roots.push(self.reducer_state());
		Ok(())
	}

	pub fn storage(&self) -> &OverlayBlockStorage<S> {
		&self.reducer_storage
	}

	pub fn state(&self) -> Option<Cid> {
		*self.reducer.state()
	}

	pub fn reducer_state(&self) -> CoReducerState {
		CoReducerState::new(*self.reducer.state(), self.reducer.heads().clone())
	}

	pub fn take_new_roots(&mut self) -> Vec<CoReducerState> {
		take(&mut self.new_roots)
	}

	// /// Commit dispatcher changes to reducer as batch.
	// pub async fn commit<D, R>(self, to_storage: &D, to: Reducer<D, R>) -> Result<Vec<CoReducerState>, anyhow::Error>
	// where
	// 	D: ExtendedBlockStorage + Send + Sync + Clone + 'static,
	// 	R: CoreResolver<S> + Send + Sync + 'static,
	// {
	// 	let mut result = Vec::new();
	// 	// // flush heads and latest state
	// 	// for root in self.new_roots.into_iter() {
	// 	// 	// heads
	// 	// 	for head in root.1.iter() {
	// 	// 		self.storage.flush(*head, Some(self.links.clone())).await?;
	// 	// 	}
	// 	// 	// state (on last)
	// 	// 	if self.new_roots.last() == root {
	// 	// 	}
	// 	// 	else {
	// 	// 		result.push(value);
	// 	// 	}
	// 	// }

	// 	// to.join(storage, heads, runtime)
	// 	Ok(result)
	// }
}
#[async_trait]
impl<S, A> CoDispatch<A> for MemoryDispatch<S, A>
where
	A: Serialize + Clone + Send + Sync + 'static,
	S: ExtendedBlockStorage + BlockStorageContentMapping + Clone + 'static,
{
	async fn dispatch(&mut self, action: &A) -> Result<Option<Cid>, anyhow::Error> {
		let state = self
			.reducer
			.push(&self.reducer_storage, self.runtime.runtime(), &self.identity, &self.core, action)
			.await?;
		self.new_roots.push(self.reducer_state());
		Ok(state)
	}
}
