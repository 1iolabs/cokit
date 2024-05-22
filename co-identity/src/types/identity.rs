use crate::DidCommPublicContext;
use co_primitives::Network;
use std::collections::BTreeSet;

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
}

/// Dynamic Identity.
pub type IdentityBox = Box<dyn Identity + Send + Sync>;
impl Identity for IdentityBox {
	fn identity(&self) -> &str {
		self.as_ref().identity()
	}

	fn public_key(&self) -> Option<Vec<u8>> {
		self.as_ref().public_key()
	}

	fn verify(&self, signature: &[u8], data: &[u8], public_key: Option<&[u8]>) -> bool {
		self.as_ref().verify(signature, data, public_key)
	}

	fn didcomm_public(&self) -> Option<DidCommPublicContext> {
		self.as_ref().didcomm_public()
	}

	fn networks(&self) -> BTreeSet<co_primitives::Network> {
		self.as_ref().networks()
	}
}
