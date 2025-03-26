use crate::{
	library::{max_reference_count::max_reference_count, to_external_cid::to_external_cid},
	types::{co_dispatch::CoDispatch, co_reducer::CoReducerContextRef},
	CoreResolver, Reducer, ReducerChangeContext, ReducerChangedHandler,
};
use async_trait::async_trait;
use cid::Cid;
use co_core_storage::StorageAction;
use co_primitives::{block_diff_added_with_parent, StoreParams, WeakCid};
use co_storage::BlockStorage;
use futures::{pin_mut, TryStreamExt};
use std::{
	collections::{BTreeMap, BTreeSet},
	mem::swap,
};

/// Reference count state in a [`co_core_storage::Storage`] core.
pub struct ReferenceWriter<D> {
	/// Parent to write the references to.
	dispatch: D,

	/// If set pin states to this key.
	pinning_key: Option<String>,

	/// The reducer storage to use.
	/// Note: This should not use any networking as we only care for local references.
	reducer_context: CoReducerContextRef,
}
impl<D> ReferenceWriter<D>
where
	D: CoDispatch<StorageAction> + 'static,
{
	pub fn new(dispatch: D, reducer_context: CoReducerContextRef, pinning_key: Option<String>) -> Self {
		Self { pinning_key, dispatch, reducer_context }
	}

	/// Update storage core from previous_state to next_state.
	pub async fn write(
		&self,
		previous_state: Option<Cid>,
		next_state: Cid,
		max_block_size: usize,
	) -> Result<Option<Cid>, anyhow::Error> {
		let mut dispatch_state = Some(next_state);
		let storage = self.reducer_context.storage(true);

		// external
		let external_next_state = to_external_cid(&storage, next_state).await;

		// calc max references per action
		let max_references = max_reference_count(max_block_size);

		// diff
		let diff = block_diff_added_with_parent(
			storage.clone(),
			previous_state,
			next_state,
			Default::default(),
			Default::default(),
		);

		// apply root reference
		if let Some(pinning_key) = &self.pinning_key {
			let action = StorageAction::PinReference(pinning_key.clone(), vec![external_next_state.into()]);
			dispatch_state = self.dispatch.dispatch(&action).await?;
		}

		// apply structural references
		let mut references = BTreeMap::<WeakCid, BTreeSet<WeakCid>>::new();
		let mut references_count = 0;
		pin_mut!(diff);
		while let Some((next_parent, next)) = diff.try_next().await? {
			if let Some(next_parent) = next_parent {
				// external
				let external_next = to_external_cid(&storage, next).await;
				let external_next_parent = to_external_cid(&storage, next_parent).await;

				// record
				references
					.entry(external_next_parent.into())
					.or_default()
					.insert(external_next.into());
				references_count += 1;

				// flush when we hit max block size
				if references_count > max_references {
					// take
					let mut next_references = Default::default();
					swap(&mut references, &mut next_references);
					references_count = 0;

					// apply
					let action = StorageAction::ReferenceStructure(next_references.into_iter().collect());
					dispatch_state = self.dispatch.dispatch(&action).await?;
				}
			}
		}
		if !references.is_empty() {
			let action = StorageAction::ReferenceStructure(references.into_iter().collect());
			dispatch_state = self.dispatch.dispatch(&action).await?;
		}
		Ok(dispatch_state)
	}
}

pub struct ReferenceWriteReducerChangedHandler<D> {
	/// The writer.
	reference_writer: ReferenceWriter<D>,

	/// The previous state. This is used as an optimizaztion to not have to walk the whole state.
	reducer_previous_state: Option<Cid>,
}
impl<D> ReferenceWriteReducerChangedHandler<D> {
	pub fn new(reference_writer: ReferenceWriter<D>, reducer_previous_state: Option<Cid>) -> Self {
		Self { reference_writer, reducer_previous_state }
	}
}
#[async_trait]
impl<D, S, R> ReducerChangedHandler<S, R> for ReferenceWriteReducerChangedHandler<D>
where
	D: CoDispatch<StorageAction> + 'static,
	S: BlockStorage + Send + Sync + Clone + 'static,
	R: CoreResolver<S> + Send + Sync + 'static,
{
	#[tracing::instrument(err(Debug), skip_all)]
	async fn on_state_changed(
		&mut self,
		_storage: &S,
		reducer: &Reducer<S, R>,
		_context: ReducerChangeContext,
	) -> Result<(), anyhow::Error> {
		// only if state has changed
		if &self.reducer_previous_state != reducer.state() {
			if let Some(next_state) = *reducer.state() {
				self.reference_writer
					.write(self.reducer_previous_state, next_state, <S::StoreParams as StoreParams>::MAX_BLOCK_SIZE)
					.await?;
			}
		}
		Ok(())
	}
}
