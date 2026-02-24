// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use cid::Cid;
use co_primitives::{MappedCid, OptionMappedCid};
use co_storage::BlockStorageContentMapping;
use futures::StreamExt;
use std::collections::BTreeSet;

/// Map internal [`Cid`] to external [`Cid`].
/// If no mapping is needed/available return the original [`Cid`].
pub async fn to_external_cid(mapping: &impl BlockStorageContentMapping, cid: Cid) -> Cid {
	if mapping.is_content_mapped().await {
		mapping.to_plain(&cid).await.unwrap_or(cid)
	} else {
		cid
	}
}

/// Map internal [`Cid`] to [`OptionMappedCid`].
pub async fn to_external_mapped(mapping: &impl BlockStorageContentMapping, internal: Cid) -> OptionMappedCid {
	if mapping.is_content_mapped().await {
		match mapping.to_plain(&internal).await {
			Some(external) => OptionMappedCid::new(internal, external),
			None => OptionMappedCid::new_unmapped(internal),
		}
	} else {
		OptionMappedCid::Unmapped(internal)
	}
}

/// Map internal [`Cid`] to [`Option<MappedCid>`].
pub async fn to_external_mapped_opt(mapping: &impl BlockStorageContentMapping, internal: Cid) -> Option<MappedCid> {
	to_external_mapped(mapping, internal).await.mapped()
}

/// Map internal [`Cid`] to [`OptionMappedCid`].
pub async fn to_external_mapped_set(
	mapping: &impl BlockStorageContentMapping,
	internal: impl IntoIterator<Item = &Cid>,
) -> BTreeSet<OptionMappedCid> {
	if mapping.is_content_mapped().await {
		futures::stream::iter(internal)
			.then(|internal| async {
				match mapping.to_plain(internal).await {
					Some(external) => OptionMappedCid::new(*internal, external),
					None => OptionMappedCid::new_unmapped(*internal),
				}
			})
			.collect()
			.await
	} else {
		internal
			.into_iter()
			.map(|internal| OptionMappedCid::new_unmapped(*internal))
			.collect()
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
