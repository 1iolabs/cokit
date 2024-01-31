use crate::{Identity, IdentityResolver, IdentityResolverError, PrivateIdentity};

/// A local identity without any actual signatures.
#[derive(Debug, Clone)]
pub struct LocalIdentity {}
impl Identity for LocalIdentity {
	fn identity(&self) -> &str {
		"local"
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
impl IdentityResolver for LocalIdentityResolver {
	fn resolve(&self, identity: &str, _public_key: Option<&[u8]>) -> Result<Box<dyn Identity>, IdentityResolverError> {
		if identity == "local" {
			return Ok(Box::new(LocalIdentity {}));
		}
		return Err(IdentityResolverError::NotFound);
	}
}
