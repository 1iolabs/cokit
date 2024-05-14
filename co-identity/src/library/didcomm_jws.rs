use super::into_didcomm_rs_header::into_didcomm_rs_header;
use crate::{DidCommHeader, SignError};
use co_primitives::Secret;
use didcomm_rs::{
	crypto::{SignatureAlgorithm, Signer},
	Message,
};

/// Create a signed JWS envelope.
///
/// # DID Comm
/// - Envelope: `signed(plaintext)`
/// - Media Type: `application/didcomm-signed+json`
pub fn didcomm_jws(private_key: Secret, header: DidCommHeader, body: &str) -> Result<String, SignError> {
	let result = Message::new()
		.didcomm_header(into_didcomm_rs_header(header))
		.body(body)
		.map_err(|e| SignError::Other(e.into()))?
		.as_jws(&SignatureAlgorithm::EdDsa)
		.sign(SignatureAlgorithm::EdDsa.signer(), private_key.divulge())
		.map_err(|e| SignError::Other(e.into()))?;
	Ok(result)
}
