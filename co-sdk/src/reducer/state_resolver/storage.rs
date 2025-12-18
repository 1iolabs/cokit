use crate::{
	library::storage_snapshots::storage_snapshots_samples,
	reducer::state_resolver::{StateResolver, StateResolverContext},
	CoReducer, CoReducerState,
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

/// Tries to resolve states from the CO storage core.
///
/// # Implementation
/// Currently we only load state from storage core on initialize.
/// Newly produced states will be managed in memory by the [`super::StaticStateResolver`].
pub struct StorageStateResolver<S> {
	parent: CoReducer,
	co: CoId,
	snapshots: Vec<CoReducerState>,
	threshold: usize,
	index: HashMap<BTreeSet<Cid>, Cid>,
	_s: PhantomData<S>,
}
impl<S> Debug for StorageStateResolver<S> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("StorageStateResolver").finish()
	}
}
impl<S> StorageStateResolver<S> {
	pub fn new(parent: CoReducer, co: CoId) -> Self {
		Self { parent, co, snapshots: Default::default(), index: Default::default(), threshold: 100, _s: PhantomData }
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
impl<S> StateResolver<S> for StorageStateResolver<S>
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

	/// Initialize the resolver.
	async fn initialize(&mut self, storage: &S) -> Result<(), anyhow::Error> {
		// load sampled snapshots
		self.snapshots = storage_snapshots_samples(
			self.parent.storage(),
			self.parent.co_state().await,
			&self.co,
			storage.clone(),
			self.threshold,
		)
		.await?;
		self.rebuild_index();

		// result
		Ok(())
	}
}
