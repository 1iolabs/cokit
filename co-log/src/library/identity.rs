pub trait Identity {
	/// The identities identifier (who).
	fn identity(&self) -> &str;

	/// Public key of the identity if it need to be referenced with the message.
	fn public_key(&self) -> Option<Vec<u8>>;

	/// Sign data and return the signature as bytes (only signature without input data).
	fn sign(&self, data: &[u8]) -> Vec<u8>;

	/// Verify signature with this identity.
	fn verify(&self, signature: &[u8], data: &[u8], public_key: Option<&[u8]>) -> bool;
}
