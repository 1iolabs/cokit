use co_storage::BlockStorageContentMapping;
use futures::{StreamExt, TryStreamExt};
use libipld::Cid;
use std::collections::BTreeSet;

pub async fn to_plain<M: BlockStorageContentMapping + Send + Sync + 'static>(
	mapping: &Option<M>,
	force_mapping: bool,
	mapped: impl IntoIterator<Item = Cid>,
) -> Result<BTreeSet<Cid>, Cid> {
	futures::stream::iter(mapped)
		.then(|head| to_plain_one(mapping, force_mapping, head))
		.try_collect()
		.await
}

pub async fn to_plain_one<M: BlockStorageContentMapping + Send + Sync + 'static>(
	mapping: &Option<M>,
	force_mapping: bool,
	mapped: Cid,
) -> Result<Cid, Cid> {
	match mapping {
		Some(mapping) => match mapping.to_plain(&mapped).await {
			Some(cid) => Ok(cid),
			None if force_mapping => Err(mapped),
			None => Ok(mapped),
		},
		None => Ok(mapped),
	}
}
