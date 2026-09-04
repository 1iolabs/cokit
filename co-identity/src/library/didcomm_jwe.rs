// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

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
use std::mem::take;

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
	let mut header = header;
	let fields = take(&mut header.fields);
	let mut message = Message::new()
		.didcomm_header(into_didcomm_rs_header(header))
		.body(body)
		.map_err(|e| SignError::Other(e.into()))?;
	for (key, value) in fields {
		message = message.add_header_field(key, value);
	}
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

	// header
	let mut header = from_didcomm_rs_header(message.get_didcomm_header().clone());
	for (key, value) in message.get_application_params() {
		header.fields.insert(key.to_owned(), value.to_owned());
	}

	// result
	Ok((header, message.get_body().map_err(|e| ReceiveError::InvalidArgument(e.into()))?))
}

#[cfg(test)]
mod tests {
	use super::{didcomm_jwe, didcomm_jwe_receive};
	use crate::{DidCommHeader, DidKeyIdentity, DidKeyIdentityResolver, Identity};
	use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
	use chacha20poly1305::{
		aead::{Aead, Payload},
		KeyInit, XChaCha20Poly1305, XNonce,
	};
	use didcomm_rs::{
		crypto::{CryptoAlgorithm, Cypher},
		Jwe, Message,
	};
	use rand::RngCore;
	use serde_json::Value;

	/// First 32 bytes of the ChaCha20 keystream under an all-zero key and nonce.
	const REPRODUCIBLE_CEK: &str = "76b8e0ada0f13d90405d6ae55386bd28bdd219b8a08ded1aa836efcc8b770dc7";

	fn seal(from: &DidKeyIdentity, to: &DidKeyIdentity, body: &str) -> String {
		let header = DidCommHeader {
			id: "cek".to_owned(),
			from: Some(from.identity().to_owned()),
			to: vec![to.identity().to_owned()].into_iter().collect(),
			message_type: "test".to_owned(),
			..Default::default()
		};
		didcomm_jwe(from.private_key_bytes(), to.public_key_bytes(), header, body).unwrap()
	}

	#[test]
	fn content_encryption_key_is_not_reproducible() {
		let from = DidKeyIdentity::generate_x25519(Some(&[21; 32]));
		let to = DidKeyIdentity::generate_x25519(Some(&[22; 32]));
		let message = seal(&from, &to, "\"payload\"");

		let cek = hex::decode(REPRODUCIBLE_CEK).expect("constant is valid hex");
		assert!(
			Message::decrypt(message.as_bytes(), CryptoAlgorithm::XC20P.decrypter(), &cek).is_err(),
			"envelope decrypted with a publicly reproducible content encryption key"
		);
	}

	#[tokio::test]
	async fn seals_from_identical_input_do_not_share_a_key() {
		let from = DidKeyIdentity::generate_x25519(Some(&[23; 32]));
		let to = DidKeyIdentity::generate_x25519(Some(&[24; 32]));
		let captured: Value = serde_json::from_str(&seal(&from, &to, "\"payload\"")).unwrap();
		let mut target: Value = serde_json::from_str(&seal(&from, &to, "\"payload\"")).unwrap();

		// the recipient entry wraps the sealing key, so it only opens the other envelope when both
		// seals produced the same key
		target["header"] = captured["header"].clone();
		target["encrypted_key"] = captured["encrypted_key"].clone();
		let forged = serde_json::to_string(&target).unwrap();

		// rejection at content decryption, rather than at the unwrap, is what shows the unwrapped
		// key differs from the one that sealed this envelope
		let error = didcomm_jwe_receive(to.private_key_bytes(), &DidKeyIdentityResolver::new(), &forged)
			.await
			.expect_err("a captured recipient entry opened a different envelope");
		let error = format!("{error:?}");
		assert!(error.contains("plugged cryptography failure"), "unexpected rejection: {error}");
	}

	#[tokio::test]
	async fn forged_payload_under_the_reproducible_key_is_rejected() {
		let from = DidKeyIdentity::generate_x25519(Some(&[25; 32]));
		let to = DidKeyIdentity::generate_x25519(Some(&[26; 32]));
		let captured: Jwe = serde_json::from_str(&seal(&from, &to, "\"payload\"")).unwrap();
		let protected = captured
			.protected
			.clone()
			.expect("a sealed envelope carries a protected header");
		let recipient = captured.recipient.clone().expect("a flat envelope carries a recipient");

		// the forgery keeps the captured header and recipient entry, and supplies content encrypted
		// under the key every pre-fix envelope used
		let attacker = Message::new()
			.from(from.identity())
			.to(&[to.identity()])
			.body("\"forged\"")
			.unwrap();
		let aad = URL_SAFE_NO_PAD.encode(serde_json::to_string(&protected).unwrap());
		let key: [u8; 32] = hex::decode(REPRODUCIBLE_CEK).unwrap().try_into().unwrap();
		let mut nonce = [0u8; 24];
		rand::rngs::OsRng.fill_bytes(&mut nonce);
		let sealed = XChaCha20Poly1305::new((&key).into())
			.encrypt(
				XNonce::from_slice(&nonce),
				Payload { msg: &serde_json::to_vec(&attacker).unwrap(), aad: aad.as_bytes() },
			)
			.unwrap();
		let (ciphertext, tag) = sealed.split_at(sealed.len() - 16);

		let forged = serde_json::to_string(&Jwe::new_flat(
			None,
			recipient,
			ciphertext,
			Some(protected),
			Some(tag),
			Some(URL_SAFE_NO_PAD.encode(nonce)),
		))
		.unwrap();

		let received = didcomm_jwe_receive(to.private_key_bytes(), &DidKeyIdentityResolver::new(), &forged).await;
		assert!(received.is_err(), "a payload forged under the reproducible key was accepted");
	}

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
