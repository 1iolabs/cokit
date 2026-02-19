// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

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
