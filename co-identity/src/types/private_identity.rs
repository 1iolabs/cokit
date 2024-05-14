use super::didcomm::context::DidCommPublicContext;
use crate::{DidCommPrivateContext, Identity};
use std::fmt::Debug;

/// Private identity representation.
pub trait PrivateIdentity: Identity {
	/// Sign data and return the signature as bytes (only signature without input data).
	fn sign(&self, data: &[u8]) -> Result<Vec<u8>, SignError>;

	/// Private DIDComm context.
	fn didcomm_private(&self) -> Option<DidCommPrivateContext>;
}

/// Dynamic Private Identity.
pub type PrivateIdentityBox = Box<dyn PrivateIdentity + Send + Sync>;

impl Identity for PrivateIdentityBox {
	fn identity(&self) -> &str {
		self.as_ref().identity()
	}

	fn public_key(&self) -> Option<Vec<u8>> {
		self.as_ref().public_key()
	}

	fn verify(&self, signature: &[u8], data: &[u8], public_key: Option<&[u8]>) -> bool {
		self.as_ref().verify(signature, data, public_key)
	}

	fn didcomm_public(&self) -> Option<DidCommPublicContext> {
		self.as_ref().didcomm_public()
	}
}
impl PrivateIdentity for PrivateIdentityBox {
	fn sign(&self, data: &[u8]) -> Result<Vec<u8>, SignError> {
		self.as_ref().sign(data)
	}

	fn didcomm_private(&self) -> Option<DidCommPrivateContext> {
		self.as_ref().didcomm_private()
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
