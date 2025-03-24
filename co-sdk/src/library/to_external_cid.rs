use cid::Cid;
use co_storage::BlockStorageContentMapping;
use futures::StreamExt;
use std::collections::BTreeSet;

/// Map internal [`Cid`] to external [`Cid`].
/// If no mapping is needed/available return the original [`Cid`].
pub async fn to_external_cid(mapping: &impl BlockStorageContentMapping, cid: Cid) -> Cid {
	mapping.to_plain(&cid).await.unwrap_or(cid)
}

/// Map internal [`Cid`] to external [`Cid`].
/// If no mapping is needed/available return the original [`Cid`].
pub async fn to_external_cid_opt(mapping: &impl BlockStorageContentMapping, cid: Option<Cid>) -> Option<Cid> {
	if let Some(cid) = cid {
		Some(mapping.to_plain(&cid).await.unwrap_or(cid))
	} else {
		cid
	}
}

/// Map internal [`Cid`] to external [`Cid`].
/// If no mapping is needed/available return the original [`Cid`].
pub async fn to_external_cids(mapping: &impl BlockStorageContentMapping, cids: BTreeSet<Cid>) -> BTreeSet<Cid> {
	futures::stream::iter(cids)
		.then(|cid| to_external_cid(mapping, cid))
		.collect()
		.await
}
