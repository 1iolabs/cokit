// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{
	library::{sample_stream::sample_stream_ordered_first_last, to_internal_cid::to_internal_cid},
	state::{query_core, Query, QueryExt},
	CoPinningKey, CoReducerState, CoRoot, CO_CORE_NAME_STORAGE,
};
use co_core_co::Co;
use co_primitives::{AnyBlockStorage, CoId, Link, OptionLink};
use co_storage::{BlockStorageContentMapping, BlockStorageExt};
use futures::{pin_mut, Stream, TryStreamExt};

/// Read all pinned CO roots from the stroage core.
/// The roots are returned from newest (first) to oldest (last).
/// This method return internal/mapped Cid's.
pub fn storage_snapshots(
	storage_core_storage: impl AnyBlockStorage,
	storage_core_state: OptionLink<Co>,
	co_id: &CoId,
	co_storage: impl AnyBlockStorage + BlockStorageContentMapping,
) -> impl Stream<Item = Result<CoReducerState, anyhow::Error>> + 'static {
	let pin = CoPinningKey::Root.to_string(co_id);
	async_stream::try_stream! {
		let pins = query_core(CO_CORE_NAME_STORAGE)
			.with_default()
			.map(|storage_core| storage_core.pins)
			.execute(&storage_core_storage, storage_core_state)
			.await?;
		if let Some(pin) = pins.get(&storage_core_storage, &pin).await? {
			let references = pin.references.reverse_stream(&storage_core_storage);
			pin_mut!(references);
			while let Some((_reference_index, reference)) = references.try_next().await? {
				let root_link: Link<CoRoot> = to_internal_cid(&co_storage, reference.cid()).await.into();
				let root = co_storage.get_value(&root_link).await?;
				yield root.into();
			}
		}
	}
}

/// Read pinned CO roots samples from the storage core.
/// The roots are returned from newest (first) to oldest (last).
pub async fn storage_snapshots_samples(
	storage_core_storage: impl AnyBlockStorage,
	storage_core_state: OptionLink<Co>,
	co_id: &CoId,
	co_storage: impl AnyBlockStorage + BlockStorageContentMapping,
	max_samples: usize,
) -> Result<Vec<CoReducerState>, anyhow::Error> {
	sample_stream_ordered_first_last(
		storage_snapshots(storage_core_storage, storage_core_state, co_id, co_storage),
		max_samples,
	)
	.await
}
