use super::didcomm::context::DidCommPublicContext;
use crate::{DidCommPrivateContext, Identity};
use std::{fmt::Debug, sync::Arc};

/// Private identity representation.
pub trait PrivateIdentity: Identity {
	/// Sign data and return the signature as bytes (only signature without input data).
	fn sign(&self, data: &[u8]) -> Result<Vec<u8>, SignError>;

	/// Private DIDComm context.
	fn didcomm_private(&self) -> Option<DidCommPrivateContext>;

	fn try_didcomm_private(&self) -> Result<DidCommPrivateContext, anyhow::Error> {
		Ok(self
			.didcomm_private()
			.ok_or(anyhow::anyhow!("unsupported identity: no private didcomm context: {}", self.identity()))?)
	}

	fn boxed(self) -> PrivateIdentityBox
	where
		Self: Sized + Clone + Send + Sync + 'static,
	{
		PrivateIdentityBox::new(self)
	}
}

/// Dynamic Private Identity.
#[derive(Clone)]
pub struct PrivateIdentityBox {
	identity: Arc<dyn PrivateIdentity + Send + Sync + 'static>,
}
impl PrivateIdentityBox {
	pub fn new<I: PrivateIdentity + Send + Sync + 'static>(identity: I) -> Self {
		Self { identity: Arc::new(identity) }
	}
}
impl Debug for PrivateIdentityBox {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("PrivateIdentity")
			.field("did", &self.identity.identity())
			.finish()
	}
}
impl Identity for PrivateIdentityBox {
	fn identity(&self) -> &str {
		self.identity.identity()
	}

	fn public_key(&self) -> Option<Vec<u8>> {
		self.identity.public_key()
	}

	fn verify(&self, signature: &[u8], data: &[u8], public_key: Option<&[u8]>) -> bool {
		self.identity.verify(signature, data, public_key)
	}

	fn didcomm_public(&self) -> Option<DidCommPublicContext> {
		self.identity.didcomm_public()
	}

	fn networks(&self) -> std::collections::BTreeSet<co_primitives::Network> {
		self.identity.networks()
	}
}
impl PrivateIdentity for PrivateIdentityBox {
	fn sign(&self, data: &[u8]) -> Result<Vec<u8>, SignError> {
		self.identity.sign(data)
	}

	fn didcomm_private(&self) -> Option<DidCommPrivateContext> {
		self.identity.didcomm_private()
	}

	fn boxed(self) -> PrivateIdentityBox
	where
		Self: Sized + Clone + Send + Sync + 'static,
	{
		self.clone()
	}
}

#[derive(Debug, thiserror::Error)]
pub enum SignError {
	/// Unauthorized error.
	/// Ususally this means that this identity has no private key.
	#[error("Unauthorized")]
	Unauthorized,

	/// Invalid argument has been supplied.
	#[error("Invalid argument")]
	InvalidArgument(#[source] anyhow::Error),

	/// Other error
	#[error("Signature failed")]
	Other(#[source] anyhow::Error),
}
