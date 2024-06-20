use super::into_didcomm_rs_header::{from_didcomm_rs_header, into_didcomm_rs_header};
use crate::{
	types::didcomm::context::DidCommContext, DidCommHeader, DidKeyIdentity, Identity, IdentityResolver, ReceiveError,
	SignError,
};
use anyhow::anyhow;
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
/// # DID Comm
/// - Envelope: `authcrypt(plaintext)`
/// - Media Type: `application/didcomm-encrypted+json`
///
/// See: https://identity.foundation/didcomm-messaging/spec/#message-headers
pub fn didcomm_jwe(
	from_key_agreement_private_key: Secret,
	to_key_agreement_public_key: Vec<u8>,
	header: DidCommHeader,
	body: &str,
) -> Result<String, SignError> {
	let message = Message::new()
		.didcomm_header(into_didcomm_rs_header(header))
		.body(body)
		.map_err(|e| SignError::Other(e.into()))?;
	let signer = DidKeyIdentity::generate(None);
	let result = message
		.as_flat_jwe(&CryptoAlgorithm::XC20P, Some(to_key_agreement_public_key.clone()))
		.kid(&hex::encode(signer.public_key_bytes()))
		.seal_signed(
			from_key_agreement_private_key.divulge(),
			Some(vec![Some(to_key_agreement_public_key.clone())]),
			SignatureAlgorithm::EdDsa,
			signer.private_key_bytes().divulge(),
		)
		.map_err(|e| SignError::Other(e.into()))?;
	Ok(result)
}

pub async fn didcomm_jwe_receive<R: IdentityResolver>(
	key_agreement_private_key: Secret,
	resolver: &R,
	incoming: &str,
) -> Result<(DidCommHeader, String), ReceiveError> {
	let jwe: Jwe = serde_json::from_str(incoming).map_err(|e| ReceiveError::UnknownFormat(e.into()))?;

	// we expect the jwe signed with a one-time key
	let skid = jwe.get_skid().ok_or_else(|| ReceiveError::MissingSigningKeyId)?;

	// resolve
	let skid_identity = resolver
		.resolve(&skid)
		.await
		.map_err(|err| ReceiveError::ResolveDidFailed(skid.clone(), err.into()))?;
	let skid_context = match skid_identity.didcomm_public() {
		Some(c) => c,
		None => {
			return Err(ReceiveError::BadDid(skid.clone(), anyhow!("No didcomm context")));
		},
	};

	// try recv
	let message = Message::receive(
		incoming,
		Some(key_agreement_private_key.divulge()),
		Some(
			skid_context
				.key_agreement()
				.public_key_bytes()
				.map_err(ReceiveError::InvalidArgument)?,
		),
		None,
	)
	.map_err(|e| ReceiveError::Decrypt(e.into()))?;

	// result
	Ok((
		from_didcomm_rs_header(message.get_didcomm_header().clone()),
		message.get_body().map_err(|e| ReceiveError::InvalidArgument(e.into()))?,
	))
}

#[cfg(test)]
mod tests {
	use super::{didcomm_jwe, didcomm_jwe_receive};
	use crate::{DidCommHeader, DidKeyIdentity, DidKeyIdentityResolver, Identity};

	#[tokio::test]
	async fn smoke() {
		let from = DidKeyIdentity::generate_x25519(Some(&[1; 32]));
		let to = DidKeyIdentity::generate_x25519(Some(&[2; 32]));
		let other = DidKeyIdentity::generate(Some(&[3; 32]));

		// create
		let header = DidCommHeader {
			id: "test".to_owned(),
			from: Some(from.identity().to_owned()),
			to: vec![to.identity().to_owned()].into_iter().collect(),
			message_type: "test".to_owned(),
			..Default::default()
		};
		let message = didcomm_jwe(from.private_key_bytes(), to.public_key_bytes(), header, "null").unwrap();

		// receive
		let (receviced_header, receviced_body) =
			didcomm_jwe_receive(to.private_key_bytes(), &DidKeyIdentityResolver::new(), &message)
				.await
				.unwrap();
		assert_eq!("test", receviced_header.id);
		assert_eq!("null", receviced_body);

		// receive other
		let received_other =
			didcomm_jwe_receive(other.private_key_bytes(), &DidKeyIdentityResolver::new(), &message).await;
		assert!(received_other.is_err());
	}
}
