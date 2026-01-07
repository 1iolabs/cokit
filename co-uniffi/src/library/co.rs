use crate::{CoCid, CoError, CoPrivateIdentity};
use cid::Cid;
use co_sdk::{from_cbor, Block, BlockStorage, CoReducer, CoReducerState};
use ipld_core::ipld::Ipld;
use std::sync::Arc;

#[derive(Debug, Clone, uniffi::Object)]
pub struct Co {
	/// The CO instance.
	///
	/// # Warning
	/// This works directly by the fact that the CoReducer internally is abstracted by an ActorHandle.
	co: CoReducer,
}
#[uniffi::export]
impl Co {
	pub async fn state(&self) -> CoState {
		self.co.reducer_state().await.into()
	}

	pub async fn push(&self, identity: &CoPrivateIdentity, core: String, action: Vec<u8>) -> Result<(), Arc<CoError>> {
		let ipld: Ipld = from_cbor(&action).map_err(CoError::new_arc)?;
		let _state = self.co.push(identity.as_ref(), core, &ipld).await.map_err(CoError::new_arc)?;
		Ok(())
	}

	// pub async fn subscribe(&self, listener: Arc<dyn CoStateListener>) -> Arc<CoStateSubscription> {
	// 	self.co.reducer_state_stream();
	// }
}
impl From<CoReducer> for Co {
	fn from(value: CoReducer) -> Self {
		Self { co: value }
	}
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct CoState {
	pub state: Option<CoCid>,
	pub heads: Vec<CoCid>,
}
impl From<CoReducerState> for CoState {
	fn from(value: CoReducerState) -> Self {
		CoState { state: value.0.map(Into::into), heads: value.1.into_iter().map(Into::into).collect() }
	}
}

// #[derive(Debug, uniffi::Object)]
// pub struct CoStateSubscription {
// 	cancel: tokio
// }
// #[uniffi::export]
// impl CoStateSubscription {
// 	pub fn close(&self) {

// 	}
// }

// #[uniffi::export]
// pub trait CoStateListener: Send + Sync {
// 	fn on_change(&self, state: CoState);
// }

#[uniffi::export]
pub async fn storage_get(co: &Co, cid: &CoCid) -> Result<Vec<u8>, Arc<CoError>> {
	let cid: Cid = cid.try_into().map_err(CoError::new_arc)?;
	Ok(co.co.storage().get(&cid).await.map_err(CoError::new_arc)?.into_inner().1)
}

#[uniffi::export]
pub async fn storage_set(co: &Co, cid: &CoCid, data: Vec<u8>) -> Result<(), Arc<CoError>> {
	let cid: Cid = cid.try_into().map_err(CoError::new_arc)?;
	let block = Block::new(cid, data).map_err(CoError::new_arc)?;
	co.co.storage().set(block).await.map_err(CoError::new_arc)?;
	Ok(())
}

#[uniffi::export]
pub async fn storage_set_data(co: &Co, codec: u64, data: Vec<u8>) -> Result<CoCid, Arc<CoError>> {
	let block = Block::new_data(codec, data);
	let cid = *block.cid();
	co.co.storage().set(block).await.map_err(CoError::new_arc)?;
	Ok(cid.into())
}
