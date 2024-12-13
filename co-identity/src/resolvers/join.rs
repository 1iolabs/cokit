use crate::{
	IdentityBox, IdentityResolver, IdentityResolverBox, IdentityResolverError, PrivateIdentityBox,
	PrivateIdentityResolver, PrivateIdentityResolverBox,
};
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct JoinIdentityResolver {
	resolvers: Vec<IdentityResolverBox>,
}
impl JoinIdentityResolver {
	pub fn new(resolvers: Vec<IdentityResolverBox>) -> Self {
		Self { resolvers }
	}
}
#[async_trait]
impl IdentityResolver for JoinIdentityResolver {
	async fn resolve(&self, identity: &str) -> Result<IdentityBox, IdentityResolverError> {
		let mut last_error: Option<IdentityResolverError> = None;
		for resolver in self.resolvers.iter() {
			match resolver.resolve(identity).await {
				Ok(i) => return Ok(i),
				Err(IdentityResolverError::NotFound) => {},
				Err(e) => last_error = Some(e),
			}
		}
		return Err(last_error.unwrap_or(IdentityResolverError::NotFound));
	}
}

#[derive(Debug, Clone)]
pub struct JoinPrivateIdentityResolver {
	resolvers: Vec<PrivateIdentityResolverBox>,
}
impl JoinPrivateIdentityResolver {
	pub fn new(resolvers: Vec<PrivateIdentityResolverBox>) -> Self {
		Self { resolvers }
	}
}
#[async_trait]
impl PrivateIdentityResolver for JoinPrivateIdentityResolver {
	async fn resolve_private(&self, identity: &str) -> Result<PrivateIdentityBox, IdentityResolverError> {
		let mut last_error: Option<IdentityResolverError> = None;
		for resolver in self.resolvers.iter() {
			match resolver.resolve_private(identity).await {
				Ok(i) => return Ok(i),
				Err(IdentityResolverError::NotFound) => {},
				Err(e) => last_error = Some(e),
			}
		}
		return Err(last_error.unwrap_or(IdentityResolverError::NotFound));
	}
}
