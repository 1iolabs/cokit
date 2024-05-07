use crate::DidCommPublicContext;

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

	///// Encrypt data using public key.
	//fn public_encrypt(&self, data: &[u8], public_key: Option<&[u8]>) -> Vec<u8>;
}

/// Dynamic Identity.
pub type IdentityBox = Box<dyn Identity + Send + Sync>;
