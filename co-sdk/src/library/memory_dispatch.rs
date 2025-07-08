use crate::{
	application::memory::create_memory_reducer, reducer::core_resolver::dynamic::DynamicCoreResolver,
	types::co_dispatch::CoDispatch, CoContext, CoReducer, CoReducerState, CoStorage, DynamicCoDate, Reducer, Runtime,
	Storage,
};
use anyhow::anyhow;
use async_trait::async_trait;
use cid::Cid;
use co_actor::TaskSpawner;
use co_identity::PrivateIdentityBox;
use co_primitives::{BlockLinks, CoId, Link, ReducerAction};
use co_storage::{BlockStorageContentMapping, ExtendedBlockStorage, OverlayBlockStorage, StoreParamsBlockStorage};
use serde::Serialize;
use std::{collections::BTreeSet, marker::PhantomData, mem::take};

pub struct MemoryDispatch<A, S>
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
impl<A, S> MemoryDispatch<A, S>
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

	pub async fn new_reducer(
		context: &CoContext,
		co: &CoReducer,
		identity: PrivateIdentityBox,
		core: impl Into<String>,
	) -> Result<MemoryDispatch<A, CoStorage>, anyhow::Error> {
		MemoryDispatch::new(
			context.inner.application_storage().clone(),
			context.inner.runtime(),
			context.date().clone(),
			context.tasks(),
			co.id().clone(),
			co.reducer_state().await,
			&co.storage(),
			identity,
			core,
		)
		.await
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
	pub async fn push_reference_with_state(
		&mut self,
		action_reference: Link<ReducerAction<A>>,
		state: Cid,
		unsafe_skip_verify: bool,
	) -> Result<(), anyhow::Error> {
		// push
		if unsafe_skip_verify {
			let head = self
				.reducer
				.log_mut()
				.push(&self.reducer_storage, &self.identity, *action_reference.cid())
				.await?;
			let heads: BTreeSet<Cid> = [*head.cid()].into_iter().collect();
			self.reducer.set_reducer_state(Some(state), heads.clone());
		} else {
			let verify_state = self
				.reducer
				.push_reference(
					&self.reducer_storage,
					self.runtime.runtime(),
					&self.identity,
					action_reference.cid().into(),
				)
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

	/// Commit changes to reducer by flushing all heads and latest state then join latest.
	pub async fn commit(&mut self, links: BlockLinks, to: &CoReducer) -> Result<(), anyhow::Error> {
		// verify
		if to.id() != &self.id {
			return Err(anyhow!("Invalid reducer specified"));
		}

		// flush heads and latest state
		let roots = self.take_new_roots();
		for root in roots.iter() {
			// heads
			for head in root.1.iter() {
				self.reducer_storage.flush(*head, Some(links.clone())).await?;
			}

			// last state
			if roots.last() == Some(root) {
				if let Some(state) = root.state() {
					self.reducer_storage.flush(state, Some(links.clone())).await?;
				}
			}
		}

		// join
		to.join_state(self.reducer_state()).await?;

		Ok(())
	}
}
#[async_trait]
impl<A, S> CoDispatch<A> for MemoryDispatch<A, S>
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

#[cfg(test)]
mod tests {
	use crate::{
		library::memory_dispatch::MemoryDispatch, ApplicationBuilder, CoDispatch, CoStorage, MonotonicCoDate,
		MonotonicCoUuid, CO_CORE_NAME_CO,
	};
	use co_core_co::CoAction;
	use co_identity::PrivateIdentity;
	use co_log::EntryBlock;
	use co_primitives::{tags, BlockStorage};

	#[tokio::test]
	async fn smoke() {
		let application = ApplicationBuilder::new_memory("test")
			// .with_bunyan_logging(Some(std::env::current_dir().unwrap().join("../data/log/co.log")))
			.with_disabled_feature("co-local-encryption")
			.with_co_date(MonotonicCoDate::default())
			.with_co_uuid(MonotonicCoUuid::default())
			.without_keychain()
			.build()
			.await
			.unwrap();
		let local_co = application.local_co_reducer().await.unwrap();
		let local_co_reducer_state = local_co.reducer_state().await;

		// create memory dispatcher
		let mut memory_dispatch = MemoryDispatch::<CoAction, CoStorage>::new_reducer(
			application.co(),
			&local_co,
			application.local_identity().boxed(),
			CO_CORE_NAME_CO,
		)
		.await
		.unwrap();
		memory_dispatch
			.dispatch(&CoAction::TagsInsert { tags: tags!("hello": "world") })
			.await
			.unwrap();
		let memory_dispatch_reducer_state = memory_dispatch.reducer_state();

		// check local has not changed
		assert_eq!(local_co.reducer_state().await, local_co_reducer_state);
		assert_ne!(memory_dispatch_reducer_state, local_co_reducer_state);
	}

	#[tokio::test]
	async fn test_push_with_state() {
		let application = ApplicationBuilder::new_memory("test")
			// .with_bunyan_logging(Some(std::env::current_dir().unwrap().join("../data/log/co.log")))
			.with_disabled_feature("co-local-encryption")
			.with_co_date(MonotonicCoDate::default())
			.with_co_uuid(MonotonicCoUuid::default())
			.without_keychain()
			.build()
			.await
			.unwrap();
		let local_co = application.local_co_reducer().await.unwrap();
		let local_co_reducer_state = local_co.reducer_state().await;

		// create memory dispatcher
		let mut memory_dispatch = MemoryDispatch::<CoAction, CoStorage>::new_reducer(
			application.co(),
			&local_co,
			application.local_identity().boxed(),
			CO_CORE_NAME_CO,
		)
		.await
		.unwrap();
		memory_dispatch
			.dispatch(&CoAction::TagsInsert { tags: tags!("hello": "world") })
			.await
			.unwrap();
		let memory_dispatch_reducer_state = memory_dispatch.reducer_state();
		let memory_dispatch_entry = EntryBlock::from_block(
			memory_dispatch
				.storage()
				.get(memory_dispatch_reducer_state.heads().first().unwrap())
				.await
				.unwrap(),
		)
		.unwrap();

		// reset
		memory_dispatch.reset(local_co_reducer_state.clone()).await.unwrap();
		memory_dispatch
			.push_reference_with_state(
				memory_dispatch_entry.entry().payload.into(),
				memory_dispatch_reducer_state.state().unwrap(),
				false,
			)
			.await
			.unwrap();
		let next_memory_dispatch_reducer_state = memory_dispatch.reducer_state();

		// reset unsafe
		memory_dispatch.reset(local_co_reducer_state.clone()).await.unwrap();
		memory_dispatch
			.push_reference_with_state(
				memory_dispatch_entry.entry().payload.into(),
				memory_dispatch_reducer_state.state().unwrap(),
				true,
			)
			.await
			.unwrap();
		let next_unsafe_memory_dispatch_reducer_state = memory_dispatch.reducer_state();

		// check local has not changed
		assert_eq!(local_co.reducer_state().await, local_co_reducer_state);
		assert_eq!(next_memory_dispatch_reducer_state, memory_dispatch_reducer_state);
		assert_eq!(next_unsafe_memory_dispatch_reducer_state, memory_dispatch_reducer_state);
		assert_ne!(memory_dispatch_reducer_state, local_co_reducer_state);
	}
}
