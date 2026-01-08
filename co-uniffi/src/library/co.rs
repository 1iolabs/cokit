use crate::{CoCid, CoError, CoPrivateIdentity};
use co_sdk::{from_cbor, Block, BlockStorage, CoReducer, CoReducerState};
use ipld_core::ipld::Ipld;

#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
#[cfg_attr(feature = "frb", flutter_rust_bridge::frb(opaque))]
#[derive(Debug, Clone)]
pub struct Co {
	/// The CO instance.
	///
	/// # Warning
	/// This works directly by the fact that the CoReducer internally is abstracted by an ActorHandle.
	co: CoReducer,
}
#[cfg_attr(feature = "uniffi", uniffi::export)]
impl Co {
	pub async fn state(&self) -> CoState {
		self.co.reducer_state().await.into()
	}

	pub async fn push(&self, identity: &CoPrivateIdentity, core: String, action: Vec<u8>) -> Result<(), CoError> {
		let ipld: Ipld = from_cbor(&action).map_err(CoError::new)?;
		let _state = self.co.push(identity.as_ref(), core, &ipld).await.map_err(CoError::new)?;
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

#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
#[derive(Debug, Clone)]
pub struct CoState {
	pub state: Option<CoCid>,
	pub heads: Vec<CoCid>,
}
impl From<CoReducerState> for CoState {
	fn from(value: CoReducerState) -> Self {
		CoState { state: value.0.map(Into::into), heads: value.1.into_iter().map(Into::into).collect() }
	}
}

// #[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
// #[derive(Debug)]
// pub struct CoStateSubscription {
// 	cancel: tokio
// }
// #[cfg_attr(feature = "uniffi", uniffi::export)]
// impl CoStateSubscription {
// 	pub fn close(&self) {

// 	}
// }

// #[cfg_attr(feature = "uniffi", uniffi::export)]
// pub trait CoStateListener: Send + Sync {
// 	fn on_change(&self, state: CoState);
// }

#[cfg_attr(feature = "uniffi", uniffi::export)]
pub async fn storage_get(co: &Co, cid: &CoCid) -> Result<Vec<u8>, CoError> {
	Ok(co.co.storage().get(&cid.cid()?).await.map_err(CoError::new)?.into_inner().1)
}

#[cfg_attr(feature = "uniffi", uniffi::export)]
pub async fn storage_set(co: &Co, cid: &CoCid, data: Vec<u8>) -> Result<(), CoError> {
	let block = Block::new(cid.cid()?, data).map_err(CoError::new)?;
	co.co.storage().set(block).await.map_err(CoError::new)?;
	Ok(())
}

#[cfg_attr(feature = "uniffi", uniffi::export)]
pub async fn storage_set_data(co: &Co, codec: u64, data: Vec<u8>) -> Result<CoCid, CoError> {
	let block = Block::new_data(codec, data);
	let cid = *block.cid();
	co.co.storage().set(block).await.map_err(CoError::new)?;
	Ok(cid.into())
}
