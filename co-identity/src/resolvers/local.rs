// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{
	DidCommPrivateContext, DidCommPublicContext, Identity, IdentityBox, IdentityResolver, IdentityResolverError,
	PrivateIdentity, PrivateIdentityBox, PrivateIdentityResolver,
};
use async_trait::async_trait;
use co_primitives::Network;
use std::collections::BTreeSet;

/// A local identity without any actual signatures.
#[derive(Debug, Clone)]
pub struct LocalIdentity {
	did: String,
}
impl LocalIdentity {
	/// Device local identity. Owner of the local CO.
	pub fn device() -> Self {
		LocalIdentity { did: "did:local:device".to_owned() }
	}

	/// Device local identity.
	///
	/// # Note
	/// This has no real usage. Use only for testing.
	pub fn new(name: &str) -> Self {
		LocalIdentity { did: format!("did:local:{name}") }
	}
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

	fn didcomm_public(&self) -> Option<DidCommPublicContext> {
		None
	}

	fn networks(&self) -> BTreeSet<Network> {
		Default::default()
	}
}
impl PrivateIdentity for LocalIdentity {
	fn sign(&self, data: &[u8]) -> Result<Vec<u8>, crate::SignError> {
		Ok(data.to_vec())
	}

	fn didcomm_private(&self) -> Option<DidCommPrivateContext> {
		None
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
		Err(IdentityResolverError::NotFound)
	}

	pub fn private_identity(&self, identity: &str) -> Result<LocalIdentity, IdentityResolverError> {
		Self::into_local_identity(identity)
	}
}
#[async_trait]
impl IdentityResolver for LocalIdentityResolver {
	async fn resolve(&self, identity: &str) -> Result<IdentityBox, IdentityResolverError> {
		Ok(IdentityBox::new(Self::into_local_identity(identity)?))
	}
}
#[async_trait]
impl PrivateIdentityResolver for LocalIdentityResolver {
	async fn resolve_private(&self, identity: &str) -> Result<PrivateIdentityBox, IdentityResolverError> {
		Ok(PrivateIdentityBox::new(Self::into_local_identity(identity)?))
	}
}
