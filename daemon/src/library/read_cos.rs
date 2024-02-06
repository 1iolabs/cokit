use anyhow::Result;
use co_sdk::{BlockSerializer, BlockStorage, Co, CoStorage};
use futures::future::join_all;
use libipld::Cid;

pub async fn read_cos(storage: &CoStorage, cid: &Option<Cid>) -> Result<Vec<Result<Co>>> {
	if let Some(cid) = cid {
		let cids: Vec<Cid> = BlockSerializer::default().deserialize(&storage.get(cid).await?)?;
		let cos = cids.iter().map(|i| async {
			let r: Co = BlockSerializer::default().deserialize(&storage.get(i).await?)?;
			Ok::<Co, anyhow::Error>(r)
		});
		return Ok(join_all(cos).await)
	}
	Ok(Vec::new())
}
