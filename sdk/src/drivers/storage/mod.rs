use anyhow::Result;
use libipld::cid::Cid;
use libipld::ipld::Ipld;
use std::sync::Arc;
use thiserror::Error;

pub mod iroh;

pub type StorageType = Arc<dyn Storage + Send + Sync + 'static>;

#[async_trait::async_trait]
pub trait Storage {
    async fn get_object(&self, cid: &Cid) -> Result<Ipld>;
    async fn put_object(&self, data: &Ipld) -> Result<Cid>;
}

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("read unexpected data type")]
    UnexpectedDataType,
}
