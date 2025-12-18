use crate::{
	library::storage_snapshots::storage_snapshots_samples,
	reducer::state_resolver::{StateResolver, StateResolverContext},
	CoReducerState, ReducerChangeContext,
};
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{AnyBlockStorage, CoId};
use co_storage::BlockStorageContentMapping;
use std::{
	collections::{BTreeSet, HashMap},
	fmt::Debug,
	marker::PhantomData,
};

/// Tries to resolve states from the current CO storage core.
pub struct LocalStorageStateResolver<S> {
	co: CoId,
	snapshots: Vec<CoReducerState>,
	threshold: usize,
	index: HashMap<BTreeSet<Cid>, Cid>,
	_s: PhantomData<S>,
}
impl<S> Debug for LocalStorageStateResolver<S> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("LocalStorageStateResolver").finish()
	}
}
impl<S> LocalStorageStateResolver<S>
where
	S: AnyBlockStorage + BlockStorageContentMapping,
{
	pub fn new(co: CoId) -> Self {
		Self { co, _s: PhantomData, snapshots: Default::default(), index: Default::default(), threshold: 100 }
	}

	fn rebuild_index(&mut self) {
		self.index = self
			.snapshots
			.iter()
			.filter_map(|state| state.some().map(|(state, heads)| (heads, state)))
			.collect();
	}
}
#[async_trait]
impl<S> StateResolver<S> for LocalStorageStateResolver<S>
where
	S: AnyBlockStorage + BlockStorageContentMapping,
{
	async fn resolve_state(
		&self,
		_storage: &S,
		_context: &StateResolverContext,
		heads: &BTreeSet<Cid>,
	) -> Result<Option<(Cid, BTreeSet<Cid>)>, anyhow::Error> {
		Ok(self.index.get(heads).map(|state| (*state, heads.clone())))
	}

	async fn push_state(
		&mut self,
		storage: &S,
		change_context: &ReducerChangeContext,
		state: Cid,
		_heads: &BTreeSet<Cid>,
	) -> Result<(), anyhow::Error> {
		if change_context.is_initialize() && self.snapshots.is_empty() {
			self.snapshots =
				storage_snapshots_samples(storage.clone(), state.into(), &self.co, storage.clone(), self.threshold)
					.await?;
			self.rebuild_index();
		}
		Ok(())
	}
}
