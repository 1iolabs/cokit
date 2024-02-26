use crate::IdentityBox;

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
	fn resolve(&self, identity: &str, public_key: Option<&[u8]>) -> Result<IdentityBox, IdentityResolverError>;
}

/// Dynamic Identity Resolver.
pub type IdentityResolverBox = Box<dyn IdentityResolver + Send + Sync + 'static>;
