// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{BlockStorage, CoCid, CoContext, CoError, CoPrivateIdentity};
use co_sdk::{from_cbor, CoReducer, CoReducerState};
use ipld_core::ipld::Ipld;
use tokio_util::sync::CancellationToken;

#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
#[cfg_attr(feature = "frb", flutter_rust_bridge::frb(opaque))]
#[derive(Debug, Clone)]
pub struct Co {
	/// The context instance.
	context: CoContext,

	/// The CO instance.
	///
	/// # Warning
	/// This works directly by the fact that the CoReducer internally is abstracted by an ActorHandle.
	pub(crate) co: CoReducer,
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

	pub fn storage(&self) -> BlockStorage {
		BlockStorage::new(self.co.storage())
	}

	#[cfg_attr(feature = "frb", flutter_rust_bridge::frb(sync))]
	pub fn subscribe(&self) -> CoSubscription {
		CoSubscription { co: self.clone(), cancel: CancellationToken::new() }
	}
}
impl From<(CoContext, CoReducer)> for Co {
	fn from((context, co): (CoContext, CoReducer)) -> Self {
		Self { context, co }
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

#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
#[cfg_attr(feature = "frb", flutter_rust_bridge::frb(opaque))]
pub struct CoSubscription {
	cancel: CancellationToken,
	co: Co,
}
impl CoSubscription {
	#[cfg_attr(feature = "frb", flutter_rust_bridge::frb(sync))]
	pub fn close(&self) {
		self.cancel.cancel();
	}

	#[cfg(feature = "frb")]
	pub async fn stream(&self, sink: crate::frb_generated::StreamSink<CoState>) {
		self.co
			.context
			.handle
			.dispatch(crate::library::co_application::CoMessage::CoSubscribe(
				self.co.clone(),
				self.cancel.child_token(),
				sink,
			))
			.ok();
	}
}
impl Drop for CoSubscription {
	fn drop(&mut self) {
		self.cancel.cancel();
	}
}

// #[cfg_attr(feature = "uniffi", uniffi::export)]
// pub trait CoStateListener: Send + Sync {
// 	fn on_change(&self, state: CoState);
// }

// #[cfg_attr(feature = "uniffi", uniffi::export)]
// pub async fn storage_get(co: &Co, cid: &CoCid) -> Result<Vec<u8>, CoError> {
// 	Ok(co.co.storage().get(&cid.cid()?).await.map_err(CoError::new)?.into_inner().1)
// }

// #[cfg_attr(feature = "uniffi", uniffi::export)]
// pub async fn storage_set(co: &Co, cid: &CoCid, data: Vec<u8>) -> Result<(), CoError> {
// 	let block = Block::new(cid.cid()?, data).map_err(CoError::new)?;
// 	co.co.storage().set(block).await.map_err(CoError::new)?;
// 	Ok(())
// }

// #[cfg_attr(feature = "uniffi", uniffi::export)]
// pub async fn storage_set_data(co: &Co, codec: u64, data: Vec<u8>) -> Result<CoCid, CoError> {
// 	let block = Block::new_data(codec, data);
// 	let cid = *block.cid();
// 	co.co.storage().set(block).await.map_err(CoError::new)?;
// 	Ok(cid.into())
// }
