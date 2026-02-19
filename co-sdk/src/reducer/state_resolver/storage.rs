// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{
	library::{storage_snapshots::storage_snapshots_samples, to_external_cid::to_external_cid},
	reducer::state_resolver::{StateResolver, StateResolverContext},
	types::co_pinning_key::CoPinningKey,
	CoReducer, CoReducerState, CoRoot, ReducerChangeContext, CO_CORE_NAME_STORAGE,
};
use async_trait::async_trait;
use cid::Cid;
use co_core_storage::{PinStrategy, References, StorageAction};
use co_identity::PrivateIdentityBox;
use co_primitives::{AnyBlockStorage, CoId};
use co_storage::{BlockStorageContentMapping, BlockStorageExt};
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
	parent_identity: PrivateIdentityBox,
	pin_strategy: PinStrategy,
	co: CoId,
	snapshots: Vec<CoReducerState>,
	/// Maximum count of states to keep in memory.
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
	pub fn new(parent: CoReducer, parent_identity: PrivateIdentityBox, pin_strategy: PinStrategy, co: CoId) -> Self {
		Self {
			parent,
			parent_identity,
			pin_strategy,
			co,
			snapshots: Default::default(),
			index: Default::default(),
			threshold: 100,
			_s: PhantomData,
		}
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

	/// Push a new latest state that we calculated.
	async fn push_state(
		&mut self,
		storage: &S,
		change_context: &ReducerChangeContext,
		state: Cid,
		heads: &BTreeSet<Cid>,
	) -> Result<(), anyhow::Error> {
		// create pin upon first use
		if change_context.is_initialize() && self.snapshots.is_empty() {
			let root = CoRoot { heads: heads.clone(), state: Some(state) };
			let root_link = storage.set_serialized(&root).await?;
			let external_root_link = to_external_cid(storage, root_link).await;
			let mut references = References::new();
			references.insert(external_root_link);
			self.parent
				.push(
					&self.parent_identity,
					CO_CORE_NAME_STORAGE,
					&StorageAction::PinCreate(
						CoPinningKey::Root.to_string(&self.co),
						self.pin_strategy.clone(),
						references,
					),
				)
				.await?;
		}
		Ok(())
	}
}
