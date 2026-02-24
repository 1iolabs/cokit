// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use cid::Cid;
use co_primitives::OptionMappedCid;
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

/// Map external [`Cid`] to [`OptionMappedCid`].
/// Returns None if can not be mapped.
#[allow(dead_code)]
pub async fn to_internal_mapped(mapping: &impl BlockStorageContentMapping, external: Cid) -> Option<OptionMappedCid> {
	if mapping.is_content_mapped().await {
		mapping
			.to_mapped(&external)
			.await
			.map(|internal| OptionMappedCid::new(internal, external))
	} else {
		Some(OptionMappedCid::Unmapped(external))
	}
}

/// Map external [`Cid`] to internal [`Cid`].
/// If no mapping is needed/available return the original [`Cid`].
pub async fn to_internal_cid_opt(mapping: &impl BlockStorageContentMapping, cid: Option<Cid>) -> Option<Cid> {
	if mapping.is_content_mapped().await {
		if let Some(cid) = cid {
			Some(mapping.to_mapped(&cid).await.unwrap_or(cid))
		} else {
			cid
		}
	} else {
		cid
	}
}

/// Map external [`Cid`] to internal [`Cid`].
/// If [`Cid`] could not be mapped return [`None`].
/// If mapping is not enabled return the original Cids.
pub async fn to_internal_cid_opt_force(mapping: &impl BlockStorageContentMapping, cid: Option<Cid>) -> Option<Cid> {
	if mapping.is_content_mapped().await {
		if let Some(cid) = cid {
			mapping.to_mapped(&cid).await
		} else {
			None
		}
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

/// Map external [`Cid`] to internal [`Cid`].
/// If some [`Cid`] could not be mapped return [`None`].
/// If mapping is not enabled return the original Cids.
pub async fn to_internal_cids_opt_force(
	mapping: &impl BlockStorageContentMapping,
	cids: BTreeSet<Cid>,
) -> Option<BTreeSet<Cid>> {
	if mapping.is_content_mapped().await {
		let mut result = BTreeSet::new();
		for cid in cids {
			result.insert(to_internal_cid_opt_force(mapping, Some(cid)).await?);
		}
		Some(result)
	} else {
		Some(cids)
	}
}
