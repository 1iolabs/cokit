use cid::Cid;
use co_storage::{BlockStorageContentMapping, StorageError};
use futures::{StreamExt, TryStreamExt};
use std::collections::BTreeSet;

/// Map internal [`Cid`] to external [`Cid`].
/// If no mapping is needed/available return the original [`Cid`].
pub async fn to_external_cid(mapping: &impl BlockStorageContentMapping, cid: Cid) -> Result<Cid, StorageError> {
	Ok(mapping.to_plain(&cid).await.unwrap_or(cid))
}

/// Map internal [`Cid`] to external [`Cid`].
/// If no mapping is needed/available return the original [`Cid`].
pub async fn to_external_cids(
	mapping: &impl BlockStorageContentMapping,
	cids: BTreeSet<Cid>,
) -> Result<BTreeSet<Cid>, StorageError> {
	Ok(futures::stream::iter(cids)
		.then(|cid| to_external_cid(mapping, cid))
		.try_collect()
		.await?)
}
