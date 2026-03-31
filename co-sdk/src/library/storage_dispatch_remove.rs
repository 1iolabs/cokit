// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{library::max_reference_count::max_reference_count, types::co_dispatch::CoDispatch};
use co_core_storage::{BlockInfo, StorageAction};
use co_primitives::{OptionMappedCid, WeakCid};
use futures::{pin_mut, Stream, StreamExt};
use std::collections::BTreeSet;

/// Apply removes in `overlay_storage` to `storage` and storage core (using `dispatch`).
pub async fn storage_dispatch_remove(
	dispatch: &mut impl CoDispatch<StorageAction>,
	info: BlockInfo,
	removed_blocks: impl Stream<Item = OptionMappedCid>,
	max_block_size: usize,
) -> Result<(), anyhow::Error> {
	let max_references = max_reference_count(max_block_size);
	let mut remove = BTreeSet::<WeakCid>::new();
	pin_mut!(removed_blocks);
	while let Some(cid) = removed_blocks.next().await {
		remove.insert(cid.external().into());
		if remove.len() > max_references {
			let mut next_remove = Default::default();
			std::mem::swap(&mut remove, &mut next_remove);
			let action = StorageAction::Remove(info.clone(), next_remove, false);
			dispatch.dispatch(&action).await?;
		}
	}
	if !remove.is_empty() {
		let action = StorageAction::Remove(info, remove, false);
		dispatch.dispatch(&action).await?;
	}
	Ok(())
}
