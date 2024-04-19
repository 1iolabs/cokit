use crate::{
	Identity, IdentityResolver, IdentityResolverError, PrivateIdentity, PrivateIdentityBox, PrivateIdentityResolver,
};
use async_trait::async_trait;

/// A local identity without any actual signatures.
#[derive(Debug, Clone)]
pub struct LocalIdentity {
	did: String,
}
impl Identity for LocalIdentity {
	fn identity(&self) -> &str {
		self.did.as_str()
	}

	fn public_key(&self) -> Option<Vec<u8>> {
		None
	}

	fn verify(&self, _signature: &[u8], _data: &[u8], _public_key: Option<&[u8]>) -> bool {
		true
	}
}
impl PrivateIdentity for LocalIdentity {
	fn sign(&self, data: &[u8]) -> Result<Vec<u8>, crate::SignError> {
		Ok(data.to_vec())
	}
}

#[derive(Debug, Clone, Default)]
pub struct LocalIdentityResolver {}
impl LocalIdentityResolver {
	pub fn new() -> Self {
		Self {}
	}

	fn into_local_identity(identity: &str) -> Result<LocalIdentity, IdentityResolverError> {
		if identity.starts_with("did:local:") {
			return Ok(LocalIdentity { did: identity.to_owned() });
		}
		return Err(IdentityResolverError::NotFound);
	}

	pub fn private_identity(&self, identity: &str) -> Result<LocalIdentity, IdentityResolverError> {
		Ok(Self::into_local_identity(identity)?)
	}
}
#[async_trait]
impl IdentityResolver for LocalIdentityResolver {
	async fn resolve(
		&self,
		identity: &str,
		_public_key: Option<&[u8]>,
	) -> Result<Box<dyn Identity + Send + Sync>, IdentityResolverError> {
		Ok(Box::new(Self::into_local_identity(identity)?))
	}
}
#[async_trait]
impl PrivateIdentityResolver for LocalIdentityResolver {
	async fn resolve_private(
		&self,
		identity: &str,
		_public_key: Option<&[u8]>,
	) -> Result<PrivateIdentityBox, IdentityResolverError> {
		Ok(Box::new(Self::into_local_identity(identity)?))
	}
}
