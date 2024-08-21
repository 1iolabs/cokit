use crate::DidCommPublicContext;
use co_primitives::Network;
use std::{collections::BTreeSet, fmt::Debug, sync::Arc};

/// Identity representation.
pub trait Identity {
	/// The identities identifier (who; DID).
	fn identity(&self) -> &str;

	/// Public key of the identity if it need to be referenced with the message.
	fn public_key(&self) -> Option<Vec<u8>>;

	/// Verify signature with this identity.
	fn verify(&self, signature: &[u8], data: &[u8], public_key: Option<&[u8]>) -> bool;

	/// Public DIDComm context.
	fn didcomm_public(&self) -> Option<DidCommPublicContext>;

	/// Get Networks where we can (possibly) reach the identity.
	fn networks(&self) -> BTreeSet<Network>;

	fn try_didcomm_public(&self) -> Result<DidCommPublicContext, anyhow::Error> {
		Ok(self
			.didcomm_public()
			.ok_or(anyhow::anyhow!("unsupported identity: no public didcomm context: {}", self.identity()))?)
	}

	fn boxed(self) -> IdentityBox
	where
		Self: Sized + Clone + Send + Sync + 'static,
	{
		IdentityBox::new(self)
	}
}

/// Dynamic Identity.
#[derive(Clone)]
pub struct IdentityBox {
	identity: Arc<dyn Identity + Send + Sync + 'static>,
}
impl IdentityBox {
	pub fn new<I: Identity + Send + Sync + 'static>(identity: I) -> Self {
		Self { identity: Arc::new(identity) }
	}
}
impl Debug for IdentityBox {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Identity").field("did", &self.identity.identity()).finish()
	}
}
impl Identity for IdentityBox {
	fn identity(&self) -> &str {
		self.identity.identity()
	}

	fn public_key(&self) -> Option<Vec<u8>> {
		self.identity.public_key()
	}

	fn verify(&self, signature: &[u8], data: &[u8], public_key: Option<&[u8]>) -> bool {
		self.identity.verify(signature, data, public_key)
	}

	fn didcomm_public(&self) -> Option<DidCommPublicContext> {
		self.identity.didcomm_public()
	}

	fn networks(&self) -> BTreeSet<co_primitives::Network> {
		self.identity.networks()
	}
}
