use crate::IdentityBox;
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum IdentityResolverError {
	/// Identity not found.
	/// Ususally this means that the resolver is not capable of resolving this identity.
	/// Therefore this is not retryable.
	#[error("Identity not found")]
	NotFound,

	/// Other error.
	/// This is possible retryable.
	#[error("Resolve Identitiy failed")]
	Other(#[from] anyhow::Error),
}

#[async_trait]
pub trait IdentityResolver {
	async fn resolve(&self, identity: &str) -> Result<IdentityBox, IdentityResolverError>;

	fn boxed(self) -> IdentityResolverBox
	where
		Self: Sized + Clone + Send + Sync + 'static,
	{
		IdentityResolverBox::new(self)
	}
}

/// Dynamic Identity Resolver.
pub struct IdentityResolverBox {
	resolver: Arc<dyn IdentityResolver + Send + Sync + 'static>,
}
impl IdentityResolverBox {
	pub fn new<R: IdentityResolver + Clone + Send + Sync + 'static>(resolver: R) -> Self {
		Self { resolver: Arc::new(resolver) }
	}
}
#[async_trait]
impl IdentityResolver for IdentityResolverBox {
	async fn resolve(&self, identity: &str) -> Result<IdentityBox, IdentityResolverError> {
		self.resolver.resolve(identity).await
	}
}
impl Clone for IdentityResolverBox {
	fn clone(&self) -> Self {
		Self { resolver: self.resolver.clone() }
	}
}
