use crate::{IdentityBox, IdentityResolver, IdentityResolverBox, IdentityResolverError};

pub struct JoinIdentityResolver {
	resolvers: Vec<IdentityResolverBox>,
}
impl JoinIdentityResolver {
	pub fn new(resolvers: Vec<IdentityResolverBox>) -> Self {
		Self { resolvers }
	}
}
impl IdentityResolver for JoinIdentityResolver {
	fn resolve(&self, identity: &str, public_key: Option<&[u8]>) -> Result<IdentityBox, IdentityResolverError> {
		let mut last_error: Option<IdentityResolverError> = None;
		for resolver in self.resolvers.iter() {
			match resolver.resolve(identity, public_key) {
				Ok(i) => return Ok(i),
				Err(IdentityResolverError::NotFound) => {},
				Err(e) => last_error = Some(e),
			}
		}
		return Err(last_error.unwrap_or(IdentityResolverError::NotFound));
	}
}
