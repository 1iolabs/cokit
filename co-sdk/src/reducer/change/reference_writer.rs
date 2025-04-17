use crate::{
	library::{max_reference_count::max_reference_count, to_external_cid::to_external_cid},
	types::co_dispatch::CoDispatch,
	CoreResolver, Reducer, ReducerChangeContext, ReducerChangedHandler,
};
use async_trait::async_trait;
use cid::Cid;
use co_core_storage::StorageAction;
use co_primitives::{
	block_diff_added_with_parent, BlockDiffFollow, BlockStorageSettings, CloneWithBlockStorageSettings, CoReference,
	KnownMultiCodec, MultiCodec, StoreParams, WeakCid,
};
use co_storage::{BlockStorage, BlockStorageContentMapping, BlockStorageExt, ExtendedBlockStorage, StorageError};
use futures::{pin_mut, TryStreamExt};
use serde::de::IgnoredAny;
use std::{
	collections::{BTreeMap, BTreeSet},
	mem::swap,
};

/// Reference count state in a [`co_core_storage::Storage`] core.
/// Update storage core from previous_state to next_state.
///
/// # Args
/// - `storage` - The storage to use for the diff process. The storage should not use any networking as we only care for
///   local references.
/// - `dispatch` - Parent to write the references to.
/// - `pinning_key` - If set pin states to this key.
#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), skip(dispatch, storage))]
pub async fn write_storage_references<S, D>(
	storage: S,
	dispatch: &D,
	pinning_key: Option<String>,
	previous_state: Option<Cid>,
	next_state: Cid,
	max_block_size: usize,
) -> Result<Option<Cid>, anyhow::Error>
where
	D: CoDispatch<StorageAction> + 'static,
	S: BlockStorage + BlockStorageContentMapping + Clone + 'static,
{
	let mut dispatch_state = Some(next_state);

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
		CoReferenceFollow { storage: storage.clone() },
	);

	// apply root reference
	if let Some(pinning_key) = pinning_key {
		let action = StorageAction::PinReference(pinning_key, vec![external_next_state.into()]);
		dispatch_state = dispatch.dispatch(&action).await?;
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
				dispatch_state = dispatch.dispatch(&action).await?;
			}
		}
	}
	if !references.is_empty() {
		let action = StorageAction::ReferenceStructure(references.into_iter().collect());
		dispatch_state = dispatch.dispatch(&action).await?;
	}
	Ok(dispatch_state)
}

/// Follow all except weak references.
struct CoReferenceFollow<S> {
	storage: S,
}
#[async_trait]
impl<S> BlockDiffFollow for CoReferenceFollow<S>
where
	S: BlockStorage + 'static,
{
	async fn follow(&mut self, cid: &Cid) -> Result<bool, StorageError> {
		if MultiCodec::is(cid, KnownMultiCodec::CoReference) {
			let reference: CoReference<IgnoredAny> = self.storage.get_deserialized(cid).await?;
			match reference {
				CoReference::Weak(_) => Ok(false),
				_ => Ok(true),
			}
		} else {
			Ok(true)
		}
	}
}

pub struct ReferenceWriteReducerChangedHandler<D> {
	dispatch: D,
	pinning_key: Option<String>,

	/// The previous state. This is used as an optimizaztion to not have to walk the whole state.
	reducer_previous_state: Option<Cid>,
}
impl<D> ReferenceWriteReducerChangedHandler<D> {
	pub fn new(dispatch: D, pinning_key: Option<String>, reducer_previous_state: Option<Cid>) -> Self {
		Self { dispatch, pinning_key, reducer_previous_state }
	}
}
#[async_trait]
impl<D, S, R> ReducerChangedHandler<S, R> for ReferenceWriteReducerChangedHandler<D>
where
	D: CoDispatch<StorageAction> + 'static,
	S: ExtendedBlockStorage + CloneWithBlockStorageSettings + BlockStorageContentMapping + Clone + 'static,
	R: CoreResolver<S> + Send + Sync + 'static,
{
	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), skip_all)]
	async fn on_state_changed(
		&mut self,
		storage: &S,
		reducer: &Reducer<S, R>,
		_context: ReducerChangeContext,
	) -> Result<(), anyhow::Error> {
		// only if state has changed
		if &self.reducer_previous_state != reducer.state() {
			if let Some(next_state) = *reducer.state() {
				write_storage_references(
					storage.clone_with_settings(BlockStorageSettings::new().without_networking()),
					&self.dispatch,
					self.pinning_key.clone(),
					self.reducer_previous_state,
					next_state,
					<S::StoreParams as StoreParams>::MAX_BLOCK_SIZE,
				)
				.await?;
			}
		}
		Ok(())
	}
}
