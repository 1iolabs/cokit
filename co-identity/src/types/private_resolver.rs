use crate::{IdentityResolverError, PrivateIdentityBox};
use async_trait::async_trait;

#[async_trait]
pub trait PrivateIdentityResolver {
	async fn resolve_private(
		&self,
		identity: &str,
		public_key: Option<&[u8]>,
	) -> Result<PrivateIdentityBox, IdentityResolverError>;
}

/// Dynamic Identity Resolver.
pub type PrivateIdentityResolverBox = Box<dyn PrivateIdentityResolver + Send + Sync + 'static>;
