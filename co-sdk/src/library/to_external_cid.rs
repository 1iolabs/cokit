use cid::Cid;
use co_storage::BlockStorageContentMapping;
use futures::StreamExt;
use std::collections::{BTreeMap, BTreeSet};

/// Map internal [`Cid`] to external [`Cid`].
/// If no mapping is needed/available return the original [`Cid`].
pub async fn to_external_cid(mapping: &impl BlockStorageContentMapping, cid: Cid) -> Cid {
	if mapping.is_content_mapped().await {
		mapping.to_plain(&cid).await.unwrap_or(cid)
	} else {
		cid
	}
}

/// Map internal [`Cid`] to external [`Cid`].
/// If no mapping is needed/available return the original [`Cid`].
pub async fn to_external_cid_opt(mapping: &impl BlockStorageContentMapping, cid: Option<Cid>) -> Option<Cid> {
	if mapping.is_content_mapped().await {
		if let Some(cid) = cid {
			Some(mapping.to_plain(&cid).await.unwrap_or(cid))
		} else {
			None
		}
	} else {
		cid
	}
}

/// Map internal [`Cid`] to external [`Cid`].
/// If [`Cid`] could not be mapped return [`None`].
/// If mapping is not enabled return the original Cids.
pub async fn to_external_cid_opt_force(mapping: &impl BlockStorageContentMapping, cid: Option<Cid>) -> Option<Cid> {
	if mapping.is_content_mapped().await {
		if let Some(cid) = cid {
			mapping.to_plain(&cid).await
		} else {
			None
		}
	} else {
		cid
	}
}

/// Map internal [`Cid`] to external [`Cid`].
/// If no mapping is needed/available return the original [`Cid`].
pub async fn to_external_cids(mapping: &impl BlockStorageContentMapping, cids: BTreeSet<Cid>) -> BTreeSet<Cid> {
	if mapping.is_content_mapped().await {
		futures::stream::iter(cids)
			.then(|cid| to_external_cid(mapping, cid))
			.collect()
			.await
	} else {
		cids
	}
}

/// Map internal [`Cid`] to external [`Cid`].
/// If no mapping is needed/available return the original [`Cid`].
// pub async fn to_external_cids_map(
// 	mapping: &impl BlockStorageContentMapping,
// 	cids: BTreeSet<Cid>,
// ) -> BTreeMap<Cid, Cid> {
// 	if mapping.is_content_mapped().await {
// 		futures::stream::iter(cids)
// 			.then(|cid| async move { (cid, to_external_cid(mapping, cid).await) })
// 			.collect()
// 			.await
// 	} else {
// 		cids.into_iter().map(|cid| (cid, cid)).collect()
// 	}
// }

/// Map internal [`Cid`] to external [`Cid`].
/// If some [`Cid`] could not be mapped return [`None`].
/// If mapping is not enabled return the original Cids.
pub async fn to_external_cids_opt_force(
	mapping: &impl BlockStorageContentMapping,
	cids: BTreeSet<Cid>,
) -> Option<BTreeSet<Cid>> {
	if mapping.is_content_mapped().await {
		let mut result = BTreeSet::new();
		for cid in cids {
			result.insert(to_external_cid_opt_force(mapping, Some(cid)).await?);
		}
		Some(result)
	} else {
		Some(cids)
	}
}

/// Map internal [`Cid`] to external [`Cid`].
/// If some [`Cid`] could not be mapped return [`None`].
/// If mapping is not enabled return the original Cids.
pub async fn to_external_cids_opt_map_force(
	mapping: &impl BlockStorageContentMapping,
	cids: BTreeSet<Cid>,
) -> Option<BTreeMap<Cid, Cid>> {
	if mapping.is_content_mapped().await {
		let mut result = BTreeMap::new();
		for cid in cids {
			result.insert(cid, to_external_cid_opt_force(mapping, Some(cid)).await?);
		}
		Some(result)
	} else {
		Some(cids.into_iter().map(|cid| (cid, cid)).collect())
	}
}
