use cid::Cid;
use co_storage::BlockStorageContentMapping;
use futures::StreamExt;
use std::collections::BTreeSet;

/// Map external [`Cid`] to internal [`Cid`].
/// If no mapping is needed/available return the original [`Cid`].
pub async fn to_internal_cid(mapping: &impl BlockStorageContentMapping, cid: Cid) -> Cid {
	if mapping.is_content_mapped().await {
		mapping.to_mapped(&cid).await.unwrap_or(cid)
	} else {
		cid
	}
}

/// Map external [`Cid`] to internal [`Cid`].
/// If no mapping is needed/available return the original [`Cid`].
pub async fn to_internal_cids(mapping: &impl BlockStorageContentMapping, cids: BTreeSet<Cid>) -> BTreeSet<Cid> {
	if mapping.is_content_mapped().await {
		futures::stream::iter(cids)
			.then(|cid| to_internal_cid(mapping, cid))
			.collect()
			.await
	} else {
		cids
	}
}
