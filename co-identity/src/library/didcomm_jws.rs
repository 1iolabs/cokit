// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use super::into_didcomm_rs_header::into_didcomm_rs_header;
use crate::{DidCommHeader, SignError};
use co_primitives::Secret;
use didcomm_rs::{
	crypto::{SignatureAlgorithm, Signer},
	Message,
};
use std::mem::take;

/// Create a signed JWS envelope.
///
/// # DID Comm
/// - Envelope: `signed(plaintext)`
/// - Media Type: `application/didcomm-signed+json`
pub fn didcomm_jws(
	private_key: Secret,
	public_key: &[u8],
	header: DidCommHeader,
	body: &str,
) -> Result<String, SignError> {
	let mut header = header;
	let fields = take(&mut header.fields);
	let mut message = Message::new()
		.didcomm_header(into_didcomm_rs_header(header))
		.kid(&hex::encode(public_key))
		.body(body)
		.map_err(|e| SignError::Other(e.into()))?;
	for (key, value) in fields {
		message = message.add_header_field(key, value);
	}
	let result = message
		.as_flat_jws(&SignatureAlgorithm::EdDsa)
		.sign(SignatureAlgorithm::EdDsa.signer(), private_key.divulge())
		.map_err(|e| SignError::Other(e.into()))?;
	Ok(result)
}
