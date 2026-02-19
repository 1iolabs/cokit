// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{BlockStorage, BlockStorageExt, MultiCodec};
use ipld_core::ipld::Ipld;
use std::collections::BTreeMap;

pub async fn ipld_resolve_recursive(
	storage: &impl BlockStorage,
	node: Ipld,
	keep_link: bool,
) -> Result<Ipld, anyhow::Error> {
	Ok(match node {
		Ipld::List(iplds) => {
			let mut result = Vec::new();
			for ipld in iplds.into_iter() {
				result.push(Box::pin(ipld_resolve_recursive(storage, ipld, keep_link)).await?);
			}
			Ipld::List(result)
		},
		Ipld::Map(iplds) => {
			let mut result = BTreeMap::new();
			for (key, ipld) in iplds.into_iter() {
				result.insert(key, Box::pin(ipld_resolve_recursive(storage, ipld, keep_link)).await?);
			}
			Ipld::Map(result)
		},
		Ipld::Link(cid) => {
			if MultiCodec::is_cbor(cid) {
				match storage.get_deserialized::<Ipld>(&cid).await {
					Ok(ipld) => {
						let ipld = Box::pin(ipld_resolve_recursive(storage, ipld, keep_link)).await?;
						if keep_link {
							Ipld::List(vec![Ipld::Link(cid), ipld])
						} else {
							ipld
						}
					},
					Err(err) => {
						tracing::warn!(%err, ?cid, "ipld_resolve_recursive");
						Ipld::Link(cid)
					},
				}
			} else {
				Ipld::Link(cid)
			}
		},
		_ => node,
	})
}
