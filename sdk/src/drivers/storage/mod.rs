use libipld::ipld::Ipld;
use libipld::cid::Cid;
use anyhow::Result;
use thiserror::Error;

pub mod iroh;

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
