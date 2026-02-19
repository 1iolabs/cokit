// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use super::into_didcomm_rs_header::{from_didcomm_rs_header, into_didcomm_rs_header};
use crate::{DidCommHeader, DidKeyIdentity, Identity, ReceiveError, SignError};
use co_primitives::Secret;
use didcomm_rs::{
	crypto::{CryptoAlgorithm, SignatureAlgorithm},
	Jwe, Message,
};

/// Create a encrypted JWE envelope.
///
/// This follows the recommendation to generate a new one-time signing DID just for this single call.
/// (See didcomm-messaging / message-header / from).
///
/// Envelope: `anoncrypt(plaintext)`
/// Media Type: `application/didcomm-encrypted+json`
/// See: https://identity.foundation/didcomm-messaging/spec/#message-headers
pub fn didcomm_anoncrypt(
	from_private_key: Secret,
	to_public_key: Vec<u8>,
	header: DidCommHeader,
	body: Option<&str>,
) -> Result<String, SignError> {
	let mut message = Message::new().didcomm_header(into_didcomm_rs_header(header));
	if let Some(body) = body {
		message = message.body(body).map_err(|e| SignError::Other(e.into()))?;
	}
	let signer = DidKeyIdentity::generate(None);
	let result = message
		.from(signer.identity())
		.as_jwe(&CryptoAlgorithm::XC20P, Some(to_public_key.clone()))
		.kid(&hex::encode(signer.public_key_bytes()))
		.seal_signed(
			from_private_key.divulge(),
			Some(vec![Some(signer.public_key_bytes())]),
			SignatureAlgorithm::EdDsa,
			signer.private_key_bytes().divulge(),
		)
		// .seal(from_private_key.divulge(), Some(vec![Some(to_public_key)]))
		.map_err(|e| SignError::Other(e.into()))?;
	Ok(result)
}

pub fn didcomm_anoncrypt_receive(
	to_private_key: Secret,
	incoming: &str,
) -> Result<(DidCommHeader, Option<String>), ReceiveError> {
	let jwe: Jwe = serde_json::from_str(&incoming).map_err(|e| ReceiveError::UnknownFormat(e.into()))?;

	// we expect the jwe signed with a one-time key
	let skid = jwe.get_skid().ok_or_else(|| ReceiveError::MissingSigningKeyId)?;

	// we only support did:key: as signing key
	let sign_identity = DidKeyIdentity::from_identity(&skid).map_err(|e| ReceiveError::InvalidSigningKeyId(e))?;

	// try recv
	let message =
		Message::receive(incoming, Some(to_private_key.divulge()), Some(sign_identity.public_key_bytes()), None)
			.map_err(|e| ReceiveError::Decrypt(e.into()))?;

	// result
	Ok((from_didcomm_rs_header(message.get_didcomm_header().clone()), message.get_body().ok()))
}

#[cfg(test)]
mod tests {
	use super::{didcomm_jwe, didcomm_jwe_receive};
	use crate::{DidCommHeader, DidKeyIdentity, Identity};

	#[test]
	fn smoke() {
		let from = DidKeyIdentity::generate(Some(&vec![1; 32]));
		let to = DidKeyIdentity::generate(Some(&vec![2; 32]));
		let other = DidKeyIdentity::generate(Some(&vec![3; 32]));
		println!("from: {}", from.identity());
		println!("to: {}", to.identity());

		// create
		let header = DidCommHeader {
			id: "test".to_owned(),
			from: Some(from.identity().to_owned()),
			to: vec![to.identity().to_owned()].into_iter().collect(),
			message_type: "hello".to_owned(),
			..Default::default()
		};
		let message = didcomm_jwe(from.private_key_bytes(), to.public_key_bytes(), header, None).unwrap();
		println!("message({}): {}", message.len(), message); // 2462

		// receive
		let (receviced_header, receviced_body) = didcomm_jwe_receive(to.private_key_bytes(), &message).unwrap();
		assert_eq!(None, receviced_body);
		assert_eq!("test", receviced_header.id);

		// receive other
		let received_other = didcomm_jwe_receive(other.private_key_bytes(), &message);
		assert!(received_other.is_err());
	}
}
