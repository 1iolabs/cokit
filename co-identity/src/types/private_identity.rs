use crate::Identity;

/// Private identity representation.
pub trait PrivateIdentity: Identity {
	/// Sign data and return the signature as bytes (only signature without input data).
	fn sign(&self, data: &[u8]) -> Result<Vec<u8>, SignError>;
}

/// Dynamic Private Identity.
pub type PrivateIdentityBox = Box<dyn PrivateIdentity + Send + Sync>;

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
