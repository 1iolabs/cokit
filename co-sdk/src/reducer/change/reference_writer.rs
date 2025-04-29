use crate::{
	library::{max_reference_count::max_reference_count, to_external_cid::to_external_cid},
	state::{query_core, Query},
	types::co_dispatch::CoDispatch,
	CoreResolver, Reducer, ReducerChangeContext, ReducerChangedHandler, CO_CORE_NAME_STORAGE,
};
use async_trait::async_trait;
use cid::Cid;
use co_core_storage::StorageAction;
use co_primitives::{
	block_diff_added_with_parent, BlockDiffFollow, BlockLinks, BlockStorageSettings, CloneWithBlockStorageSettings,
	CoReference, KnownMultiCodec, MultiCodec, OptionLink, StoreParams, WeakCid,
};
use co_storage::{BlockStorage, BlockStorageContentMapping, BlockStorageExt, ExtendedBlockStorage, StorageError};
use futures::{pin_mut, TryStreamExt};
use serde::de::IgnoredAny;
use std::collections::{BTreeMap, BTreeSet};

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
	dispatch: &mut D,
	block_links: BlockLinks,
	pinning_key: Option<String>,
	previous_state: Option<Cid>,
	next_state: Cid,
) -> Result<Option<Cid>, anyhow::Error>
where
	D: CoDispatch<StorageAction> + 'static,
	S: ExtendedBlockStorage + BlockStorageContentMapping + Clone + 'static,
{
	let mut dispatch_state = Some(next_state);

	// calc max references per action
	let max_references = max_reference_count(S::StoreParams::MAX_BLOCK_SIZE);

	// diff
	let diff = block_diff_added_with_parent(
		storage.clone(),
		previous_state,
		next_state,
		block_links,
		Default::default(),
		CoReferenceFollow { storage: storage.clone() },
	);

	// apply root reference
	if let Some(pinning_key) = pinning_key.clone() {
		let external_next_state = to_external_cid(&storage, next_state).await;
		let action = StorageAction::PinReference(pinning_key, vec![external_next_state.into()]);
		dispatch_state = dispatch.dispatch(&action).await?;
	}

	// apply structural references
	let mut references_indicies = BTreeMap::<WeakCid, usize>::new();
	let mut references = Vec::<(WeakCid, BTreeSet<WeakCid>)>::new();
	let mut references_count = 0;
	pin_mut!(diff);
	while let Some((next_parent, next)) = diff.try_next().await? {
		if let Some(next_parent) = next_parent {
			// skip if not exists on local disk
			//  this still will be returned by the diff as it found it
			//  but we dont need to reference it in the storage core because it is not on disk
			if !storage.exists(&next).await? {
				continue;
			}

			// external
			let external_next = WeakCid::from(to_external_cid(&storage, next).await);
			let external_next_parent = WeakCid::from(to_external_cid(&storage, next_parent).await);

			// record
			//  we need to keep the parents sorted so we dont create other parents before they was children
			if let Some(index) = references_indicies.get(&external_next_parent) {
				let entry = references.get_mut(*index).expect("index to exist");
				if entry.1.insert(external_next) {
					references_count += 1;
				}
			} else {
				references.push((external_next_parent, BTreeSet::from([external_next])));
				references_indicies.insert(external_next_parent, references.len() - 1);
				references_count += 2;
			}

			// flush when we hit max block size
			if references_count > max_references {
				// apply
				let action = StorageAction::ReferenceStructure(references);
				dispatch_state = dispatch.dispatch(&action).await?;

				// reset
				references_indicies = Default::default();
				references = Default::default();
				references_count = 0;
			}
		}
	}
	if !references.is_empty() {
		let action = StorageAction::ReferenceStructure(references);
		dispatch_state = dispatch.dispatch(&action).await?;
	}

	// log
	#[cfg(feature = "logging-verbose")]
	if let Some(pinning_key) = &pinning_key {
		let storage_state = query_core::<co_core_storage::Storage>(CO_CORE_NAME_STORAGE)
			.execute(&storage, dispatch_state.into())
			.await?;
		if let Some(pin) = storage_state.pins.get(&storage, &pinning_key).await? {
			let references = pin.references.stream(&storage).try_collect::<Vec<_>>().await?;
			tracing::trace!(?references, ?pinning_key, ?dispatch_state, "storage-pin-references");
		}
	}

	// result
	Ok(dispatch_state)
}

/// Find lastest pushed reference for a pinning key.
pub async fn lastest_storage_reference<S>(
	storage: &S,
	state: OptionLink<co_core_co::Co>,
	pinning_key: &Option<String>,
) -> Result<Option<Cid>, anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let Some(pinning_key) = pinning_key else {
		return Ok(None);
	};
	let storage_state = query_core::<co_core_storage::Storage>(CO_CORE_NAME_STORAGE)
		.execute(storage, state)
		.await?;
	let Some(pin) = storage_state.pins.get(storage, pinning_key).await? else {
		return Ok(None);
	};
	let references = pin.references.open(storage).await?;
	let stream = references.reverse_stream();
	pin_mut!(stream);
	let reference = stream.try_next().await?;
	Ok(reference.map(|(_, reference)| reference.cid()))
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
					&mut self.dispatch,
					BlockLinks::default(),
					self.pinning_key.clone(),
					self.reducer_previous_state,
					next_state,
				)
				.await?;
			}
		}
		Ok(())
	}
}
