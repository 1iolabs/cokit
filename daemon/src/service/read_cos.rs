use std::sync::Arc;
use anyhow::Result;
use co_sdk::{drivers::storage::Storage, types::co::Co};
use futures::future::join_all;
use libipld::{Cid, serde::from_ipld};

#[derive(thiserror::Error, Debug)]
pub enum ReadError {
    #[error("Failed to get item")]
    Get { source: anyhow::Error, cid: Cid },
}

pub async fn read_cos(storage: Arc<dyn Storage + Send + Sync>, cid: &Option<Cid>) -> Result<Vec<Result<Co>>> {
    if let Some(cid) = cid {
        let cids: Vec<Cid> = from_ipld(storage.get_object(cid).await?)?;
        let cos = cids.iter().map(|i| async {
            let d = storage.get_object(i).await?;
            let r: Co = from_ipld(d)?;
            Ok::<Co, anyhow::Error>(r)
        });
        return Ok(join_all(cos).await)
    }
    Ok(Vec::new())
}
