use crate::{
	types::co_reducer::CoReducerContextRef, CoReducer, CoreResolver, Reducer, ReducerChangeContext,
	ReducerChangedHandler, CO_CORE_NAME_STORAGE,
};
use async_trait::async_trait;
use cid::Cid;
use co_core_storage::StorageAction;
use co_identity::PrivateIdentityBox;
use co_primitives::{block_diff_added_with_parent, StoreParams};
use co_storage::BlockStorage;
use futures::{pin_mut, TryStreamExt};
use std::{
	collections::{BTreeMap, BTreeSet},
	mem::swap,
};

/// Reference count state in a [`co_core_storage::Storage`] core.
pub struct ReferenceWriter {
	/// Parent to write the references to.
	parent: CoReducer,

	/// Identity to use to write the references.
	identity: PrivateIdentityBox,

	/// The core name in the parent to write the refrecens to.
	storage_core_name: String,

	/// If set pin states to this key.
	pinning_key: Option<String>,

	/// The previous state. Thius is uses as an optimizaztion to not have to walk the whole state.
	reducer_previous_state: Option<Cid>,

	/// The reducer storage to use.
	/// Note: This should not use any networking as we only care for local references.
	reducer_context: CoReducerContextRef,
}
impl ReferenceWriter {
	pub fn new(
		reducer_context: CoReducerContextRef,
		reducer_previous_state: Option<Cid>,
		parent: CoReducer,
		identity: PrivateIdentityBox,
		pinning_key: Option<String>,
	) -> Self {
		Self {
			pinning_key,
			identity,
			parent,
			storage_core_name: CO_CORE_NAME_STORAGE.to_owned(),
			reducer_previous_state,
			reducer_context,
		}
	}

	pub fn with_storage_core_name(mut self, name: String) -> Self {
		self.storage_core_name = name;
		self
	}
}
#[async_trait]
impl<S, R> ReducerChangedHandler<S, R> for ReferenceWriter
where
	S: BlockStorage + Send + Sync + Clone + 'static,
	R: CoreResolver<S> + Send + Sync + 'static,
{
	#[tracing::instrument(err(Debug), skip_all)]
	async fn on_state_changed(
		&mut self,
		reducer: &Reducer<S, R>,
		_context: ReducerChangeContext,
	) -> Result<(), anyhow::Error> {
		// only if state has changed
		if &self.reducer_previous_state != reducer.state() {
			// references
			if let Some(next_state) = *reducer.state() {
				// external
				let external_next_state = self.reducer_context.to_external_cid(next_state).await?;

				// calc max references per action
				let max_references = <S::StoreParams as StoreParams>::MAX_BLOCK_SIZE / 2 / Cid::default().encoded_len();

				// diff
				let diff = block_diff_added_with_parent(
					self.reducer_context.storage(true),
					self.reducer_previous_state,
					next_state,
					Default::default(),
					Default::default(),
				);

				// apply root reference
				if let Some(pinning_key) = &self.pinning_key {
					let action = StorageAction::PinReference(pinning_key.clone(), vec![external_next_state]);
					self.parent.push(&self.identity, &self.storage_core_name, &action).await?;
				}

				// apply structural references
				let mut references = BTreeMap::<Cid, BTreeSet<Cid>>::new();
				let mut references_count = 0;
				pin_mut!(diff);
				while let Some((next_parent, next)) = diff.try_next().await? {
					if let Some(next_parent) = next_parent {
						// external
						let external_next = self.reducer_context.to_external_cid(next).await?;
						let external_next_parent = self.reducer_context.to_external_cid(next_parent).await?;

						// record
						references.entry(external_next_parent).or_default().insert(external_next);
						references_count += 1;

						// flush when we hit max block size
						if references_count > max_references {
							// take
							let mut next_references = Default::default();
							swap(&mut references, &mut next_references);
							references_count = 0;

							// apply
							let action = StorageAction::ReferenceStructure(next_references.into_iter().collect());
							self.parent.push(&self.identity, &self.storage_core_name, &action).await?;
						}
					}
				}
				if !references.is_empty() {
					let action = StorageAction::ReferenceStructure(references.into_iter().collect());
					self.parent.push(&self.identity, &self.storage_core_name, &action).await?;
				}
			}
		}
		Ok(())
	}
}
