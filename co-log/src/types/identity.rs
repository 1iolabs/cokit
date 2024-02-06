/// Identity representation.
pub trait Identity {
	/// The identities identifier (who).
	fn identity(&self) -> &str;

	/// Public key of the identity if it need to be referenced with the message.
	fn public_key(&self) -> Option<Vec<u8>>;

	/// Verify signature with this identity.
	fn verify(&self, signature: &[u8], data: &[u8], public_key: Option<&[u8]>) -> bool;
}

/// Private identity representation.
pub trait PrivateIdentity: Identity {
	/// Sign data and return the signature as bytes (only signature without input data).
	fn sign(&self, data: &[u8]) -> Result<Vec<u8>, SignError>;
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

#[derive(Debug, thiserror::Error)]
pub enum IdentityResolverError {
	/// Identity not found.
	/// Ususally this means that the resolver is not capable of resolving this identity.
	/// Therefore this is not retryable.
	#[error("Identity not found")]
	NotFound,

	/// Other error
	/// This ispossible retryable.
	#[error("Resolve Identitiy failed: {0}")]
	Other(String, #[source] anyhow::Error),
}

pub trait IdentityResolver {
	fn resolve(
		&self,
		identity: &str,
		public_key: Option<&[u8]>,
	) -> Result<Box<dyn Identity + Send + Sync>, IdentityResolverError>;
}
