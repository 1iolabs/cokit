use crate::Identity;
use std::fmt::Debug;

/// Private identity representation.
pub trait PrivateIdentity: Identity {
	/// Sign data and return the signature as bytes (only signature without input data).
	fn sign(&self, data: &[u8]) -> Result<Vec<u8>, SignError>;
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
}
impl PrivateIdentity for PrivateIdentityBox {
	fn sign(&self, data: &[u8]) -> Result<Vec<u8>, SignError> {
		self.as_ref().sign(data)
	}
}

#[derive(Debug, thiserror::Error)]
pub enum SignError {
	/// Unauthorized error.
	/// Ususally this means that this identity has no private key.
	#[error("Unauthorized")]
	Unauthorized,

	/// Other error
	#[error("Signature failed")]
	Other(#[source] anyhow::Error),
}
