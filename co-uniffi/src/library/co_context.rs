use crate::{
	library::{
		co_application::{CoApplication, CoMessage},
		co_error::CoError,
	},
	Co, CoPrivateIdentity, CoSettings,
};
use co_actor::ActorHandle;
use co_sdk::CoId;

#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
#[cfg_attr(feature = "frb", flutter_rust_bridge::frb(opaque))]
#[derive(Debug)]
pub struct CoContext {
	handle: ActorHandle<CoMessage>,
}
#[cfg_attr(feature = "uniffi", uniffi::export)]
impl CoContext {
	pub async fn open(settings: CoSettings) -> Result<Self, CoError> {
		match CoApplication::spawn(settings.clone()).await {
			Ok(handle) => Ok(CoContext { handle }),
			Err(err) => Err(CoError::new(err)),
		}
	}

	pub async fn open_co(&self, id: String) -> Result<Co, CoError> {
		let co_id = CoId::from(id);
		let co = self
			.handle
			.request(move |response| CoMessage::OpenCo(co_id, response))
			.await
			.map_err(CoError::new)?
			.map_err(CoError::new)?;
		Ok(co)
	}

	pub async fn resolve_private_identity(&self, did: String) -> Result<CoPrivateIdentity, CoError> {
		let result = self
			.handle
			.request(move |response| CoMessage::ResolvePrivateIdentity(did, response))
			.await
			.map_err(CoError::new)?
			.map_err(CoError::new)?;
		Ok(result)
	}
}

#[cfg(feature = "uniffi")]
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub async fn co_context_open(settings: &CoSettings) -> Result<CoContext, CoError> {
	match CoApplication::spawn(settings.clone()).await {
		Ok(handle) => Ok(CoContext { handle }),
		Err(err) => Err(CoError::new(err)),
	}
}
