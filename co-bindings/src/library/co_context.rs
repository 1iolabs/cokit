// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{
	library::{
		co_application::{CoApplication, CoMessage},
		co_error::CoError,
	},
	Co, CoCid, CoPrivateIdentity, CoSettings,
};
use anyhow::anyhow;
use co_actor::ActorHandle;
use co_sdk::CoId;
use std::collections::HashMap;

#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
#[cfg_attr(feature = "frb", flutter_rust_bridge::frb(opaque))]
#[derive(Debug, Clone)]
pub struct CoContext {
	pub(crate) handle: ActorHandle<CoMessage>,
}
// #[cfg_attr(feature = "uniffi", uniffi::export)]
impl CoContext {
	pub async fn open(settings: CoSettings) -> Result<Self, CoError> {
		match CoApplication::spawn(settings.clone()).await {
			Ok(handle) => Ok(CoContext { handle }),
			Err(err) => Err(CoError::new(err)),
		}
	}

	pub async fn base_path(&self) -> Result<Option<String>, CoError> {
		self.handle.request(CoMessage::BasePath).await.map_err(CoError::new)
	}

	pub async fn open_co(&self, id: String) -> Result<Co, CoError> {
		let co_id = CoId::from(id);
		let co = self
			.handle
			.request(move |response| CoMessage::CoOpen(co_id, response))
			.await
			.map_err(CoError::new)?
			.map_err(CoError::new)?;
		Ok(co)
	}

	pub async fn create_co(&self, identity: &CoPrivateIdentity, create: CreateCo) -> Result<Co, CoError> {
		let identity = identity.clone();
		let create = co_sdk::CreateCo::try_from(create)?;
		let result = self
			.handle
			.request(move |response| CoMessage::CoCreate(identity, create, response))
			.await
			.map_err(CoError::new)?
			.map_err(CoError::new)?;
		Ok(result)
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

	/// Use the first or create an identity with `name`.
	pub async fn ensure_did_key_identity(&self, name: String) -> Result<CoPrivateIdentity, CoError> {
		let result = self
			.handle
			.request(move |response| CoMessage::EnsureDidKeyIdentity(name, response))
			.await
			.map_err(CoError::new)?
			.map_err(CoError::new)?;
		Ok(result)
	}
}
impl From<ActorHandle<CoMessage>> for CoContext {
	fn from(handle: ActorHandle<CoMessage>) -> Self {
		Self { handle }
	}
}

pub struct CreateCo {
	pub id: String,
	pub name: Option<String>,
	pub public: bool,
	pub cores: HashMap<String, CreateCore>,
}
impl TryFrom<CreateCo> for co_sdk::CreateCo {
	type Error = CoError;

	fn try_from(value: CreateCo) -> Result<Self, Self::Error> {
		let mut result = co_sdk::CreateCo::new(value.id, value.name);
		if value.public {
			result = result.with_public(false);
		}
		for (core_name, core) in value.cores {
			result = if let Some(cid) = core.core_reference {
				result.with_core(&core_name, &core.core_type, cid.try_into().map_err(CoError::new)?)
			} else if let Some(bytes) = core.core_bytes {
				result.with_core_bytes(&core_name, &core.core_type, bytes)
			} else {
				return Err(CoError::new(anyhow!("Either `core_reference` or `core_bytes` must be set")));
			};
		}
		Ok(result)
	}
}

pub struct CreateCore {
	pub core_type: String,
	pub core_reference: Option<CoCid>,
	pub core_bytes: Option<Vec<u8>>,
}

#[cfg(feature = "uniffi")]
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub async fn co_context_open(settings: &CoSettings) -> Result<CoContext, CoError> {
	match CoApplication::spawn(settings.clone()).await {
		Ok(handle) => Ok(handle.into()),
		Err(err) => Err(CoError::new(err)),
	}
}
