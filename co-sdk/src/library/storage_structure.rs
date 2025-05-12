use crate::{
	library::to_external_cid::to_external_cids,
	state::{query_core, Query, QueryExt},
	types::co_dispatch::CoDispatch,
	CO_CORE_NAME_STORAGE,
};
use cid::Cid;
use co_core_co::Co;
use co_core_storage::StorageAction;
use co_primitives::{BlockLinks, OptionLink, WeakCid};
use co_storage::{BlockStorageContentMapping, ExtendedBlockStorage};
use futures::{pin_mut, TryStreamExt};
use std::{
	collections::BTreeSet,
	time::{Duration, Instant},
};

/// Resolve shallow structure.
#[tracing::instrument(err(Debug), skip(storage_core_storage, storage_core_dispatcher, storage, links))]
pub async fn storage_structure<S, D>(
	storage_core_storage: &S,
	storage_core_dispatcher: &mut impl CoDispatch<StorageAction>,
	storage_core_state: OptionLink<Co>,
	storage: &D,
	links: BlockLinks,
	max_duration: Option<Duration>,
) -> Result<(), anyhow::Error>
where
	S: ExtendedBlockStorage + BlockStorageContentMapping + Clone + 'static,
	D: ExtendedBlockStorage + BlockStorageContentMapping + Clone + 'static,
{
	let is_content_mapped = storage.is_content_mapped().await;
	let mut query_blocks_index_shallow = query_core::<co_core_storage::Storage>(CO_CORE_NAME_STORAGE)
		.with_default()
		.map(|storage_core| storage_core.blocks_index_shallow);
	let blocks_index_shallow = query_blocks_index_shallow
		.execute(storage_core_storage, storage_core_state)
		.await?;
	let shallow = blocks_index_shallow.stream(storage_core_storage);
	pin_mut!(shallow);

	// resolve
	let start = Instant::now();
	let mut num_resolved = 0;
	let mut num_visited = 0;
	let mut num_skip_exists = 0;
	let mut num_skip_no_mapping = 0;
	let mut num_skip_failure = 0;
	while let Some(cid) = shallow.try_next().await? {
		num_visited += 1;

		// skip if not exists in this storage
		if !storage.exists(&cid).await? {
			num_skip_exists += 1;
			continue;
		}

		// map (this should return None if wrong "namespace")
		let internal_cid = if is_content_mapped { storage.to_mapped(&cid).await } else { Some(cid.into()) };
		let Some(internal_cid) = internal_cid else {
			num_skip_no_mapping += 1;
			continue;
		};

		// try to resolve links
		let internal_links: BTreeSet<Cid> = if links.has_links(&internal_cid) {
			let block = match storage.get(&internal_cid).await {
				Ok(block) => block,
				Err(err) => {
					tracing::warn!(external_cid = ?cid, ?internal_cid, ?err, "storage-structure-get-failed");
					num_skip_failure += 1;
					continue;
				},
			};
			let block_links = links.links(&block)?;
			block_links.collect()
		} else {
			Default::default()
		};

		// external links
		let external_links = to_external_cids(storage, internal_links).await;

		// dispatch
		//  TODO: combine actions?
		let action =
			StorageAction::ReferenceStructure(vec![(cid, external_links.into_iter().map(WeakCid::from).collect())]);
		storage_core_dispatcher.dispatch(&action).await?;
		num_resolved += 1;

		// deadline?
		if let Some(max_duration) = max_duration {
			if max_duration < (Instant::now() - start) {
				break;
			}
		}
	}

	// log
	let duration_ms: Duration = Instant::now() - start;
	tracing::trace!(
		duration_ms = duration_ms.as_millis(),
		num_resolved,
		num_visited,
		num_skip_exists,
		num_skip_no_mapping,
		num_skip_failure,
		"storage-structure"
	);

	// result
	Ok(())
}
