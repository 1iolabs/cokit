use crate::{
	library::{
		co_application::{CoApplication, CoMessage},
		co_error::CoError,
	},
	Co, CoPrivateIdentity, CoSettings,
};
use co_actor::ActorHandle;
use co_sdk::CoId;
use std::sync::Arc;

#[derive(Debug, uniffi::Object)]
pub struct CoContext {
	handle: ActorHandle<CoMessage>,
}
#[uniffi::export]
impl CoContext {
	pub async fn open_co(&self, id: String) -> Result<Arc<Co>, Arc<CoError>> {
		let co_id = CoId::from(id);
		let co = self
			.handle
			.request(move |response| CoMessage::OpenCo(co_id, response))
			.await
			.map_err(CoError::new_arc)?
			.map_err(CoError::new_arc)?;
		Ok(Arc::new(co))
	}

	pub async fn resolve_private_identity(&self, did: String) -> Result<Arc<CoPrivateIdentity>, Arc<CoError>> {
		let result = self
			.handle
			.request(move |response| CoMessage::ResolvePrivateIdentity(did, response))
			.await
			.map_err(CoError::new_arc)?
			.map_err(CoError::new_arc)?;
		Ok(Arc::new(result))
	}
}

#[uniffi::export]
pub async fn co_context_open(settings: &CoSettings) -> Result<Arc<CoContext>, Arc<CoError>> {
	match CoApplication::spawn(settings.clone()).await {
		Ok(handle) => Ok(Arc::new(CoContext { handle })),
		Err(err) => Err(CoError::new_arc(err)),
	}
}
