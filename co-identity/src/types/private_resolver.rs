// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{IdentityResolverError, PrivateIdentityBox};
use async_trait::async_trait;
use std::{fmt::Debug, sync::Arc};

#[async_trait]
pub trait PrivateIdentityResolver: Debug {
	async fn resolve_private(&self, identity: &str) -> Result<PrivateIdentityBox, IdentityResolverError>;

	fn boxed(self) -> PrivateIdentityResolverBox
	where
		Self: Sized + Clone + Send + Sync + 'static,
	{
		PrivateIdentityResolverBox::new(self)
	}
}

/// Dynamic Identity Resolver.
#[derive(Debug, Clone)]
pub struct PrivateIdentityResolverBox {
	resolver: Arc<dyn PrivateIdentityResolver + Send + Sync + 'static>,
}
impl PrivateIdentityResolverBox {
	pub fn new<R: PrivateIdentityResolver + Clone + Send + Sync + 'static>(resolver: R) -> Self {
		Self { resolver: Arc::new(resolver) }
	}
}
#[async_trait]
impl PrivateIdentityResolver for PrivateIdentityResolverBox {
	async fn resolve_private(&self, identity: &str) -> Result<PrivateIdentityBox, IdentityResolverError> {
		self.resolver.resolve_private(identity).await
	}
}
