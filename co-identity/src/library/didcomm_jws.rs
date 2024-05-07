use crate::SignError;
use co_primitives::Secret;
use didcomm_rs::{
	crypto::{SignatureAlgorithm, Signer},
	Message,
};

/// Create a signed JWS envelope.
///
/// Envelope: `signed(plaintext)`
/// Media Type: `application/didcomm-signed+json`
pub fn didcomm_jws(private_key: Secret, body: &str) -> Result<String, SignError> {
	let result = Message::new()
		.body(body)
		.map_err(|e| SignError::Other(e.into()))?
		.as_jws(&SignatureAlgorithm::EdDsa)
		.sign(SignatureAlgorithm::EdDsa.signer(), private_key.divulge())
		.map_err(|e| SignError::Other(e.into()))?;
	Ok(result)
}
