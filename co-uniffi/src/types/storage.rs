use crate::{CoCid, CoError};
use std::sync::Arc;

#[cfg_attr(feature = "uniffi", uniffi::export)]
#[async_trait::async_trait]
pub trait CoBlockStorage: Send + Sync {
	async fn get(&self, cid: CoCid) -> Result<Vec<u8>, Arc<CoError>>;
	async fn set(&self, cid: CoCid, bytes: Vec<u8>) -> Result<(), Arc<CoError>>;
}
