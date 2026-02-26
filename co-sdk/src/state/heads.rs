// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use cid::Cid;
use co_log::{EntryBlock, Log};
use co_primitives::{AnyBlockStorage, CoId, ReducerAction};
use co_storage::BlockStorageExt;
use futures::{Stream, TryStreamExt};
use serde::de::{DeserializeOwned, IgnoredAny};
use std::collections::BTreeSet;

/// Stream entries using heads from newest to oldest.
pub fn heads_stream(
	storage: impl AnyBlockStorage,
	co: &CoId,
	heads: BTreeSet<Cid>,
) -> impl Stream<Item = Result<EntryBlock, anyhow::Error>> + 'static {
	Log::new_readonly(co.as_bytes().to_vec(), heads)
		.into_stream(storage)
		.map_err(|e| e.into())
}

/// Stream reducer using heads from newest to oldest.
pub fn heads_action_stream<A>(
	storage: impl AnyBlockStorage,
	co: &CoId,
	heads: BTreeSet<Cid>,
	core: String,
) -> impl Stream<Item = Result<ReducerAction<A>, anyhow::Error>> + 'static
where
	A: DeserializeOwned + Send + Sync + 'static,
{
	Log::new_readonly(co.as_bytes().to_vec(), heads)
		.into_stream(storage.clone())
		.map_err(anyhow::Error::from)
		.try_filter_map({
			let storage = storage.clone();
			move |entry| {
				let storage = storage.clone();
				let core = core.clone();
				async move {
					Ok({
						let action: ReducerAction<IgnoredAny> =
							storage.get_deserialized(&entry.entry().payload).await?;
						if action.core == core {
							Some(entry)
						} else {
							None
						}
					})
				}
			}
		})
		.map_ok(|entry| entry.entry().payload)
		.and_then(move |action_cid| {
			let storage = storage.clone();
			async move {
				let action: ReducerAction<A> = storage.get_deserialized(&action_cid).await?;
				Ok(action)
			}
		})
}
