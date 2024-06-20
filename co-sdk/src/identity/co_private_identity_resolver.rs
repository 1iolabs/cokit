use crate::{CoReducerFactory, DidKeyProvider, CO_CORE_NAME_KEYSTORE};
use async_trait::async_trait;
use co_identity::{IdentityResolverError, PrivateIdentityBox, PrivateIdentityResolver};

#[derive(Debug, Clone)]
pub struct CoPrivateIdentityResolver<F> {
	factory: F,
}
impl<F> CoPrivateIdentityResolver<F>
where
	F: CoReducerFactory + Clone + Sync + Send + 'static,
{
	pub fn new(factory: F) -> Self {
		Self { factory }
	}
}
#[async_trait]
impl<F> PrivateIdentityResolver for CoPrivateIdentityResolver<F>
where
	F: CoReducerFactory + Clone + Sync + Send + 'static,
{
	async fn resolve_private(&self, identity: &str) -> Result<PrivateIdentityBox, IdentityResolverError> {
		let local_co = self
			.factory
			.co_reducer(&"local".into())
			.await?
			.ok_or(IdentityResolverError::NotFound)?;
		DidKeyProvider::new(local_co, CO_CORE_NAME_KEYSTORE)
			.resolve_private(identity)
			.await
	}
}
