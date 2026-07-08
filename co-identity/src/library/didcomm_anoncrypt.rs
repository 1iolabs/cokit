// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use super::{
	didcomm_anoncrypt_envelope,
	into_didcomm_rs_header::{from_didcomm_rs_header, into_didcomm_rs_header},
};
use crate::{DidCommHeader, ReceiveError, SignError};
use anyhow::anyhow;
use co_primitives::Secret;
use didcomm_rs::Message;
use std::mem::take;

/// Create a DIDComm v2 anonymous-encryption JWE envelope.
///
/// This function does not use sender private key material. It clears the plaintext `from` header
/// before packing, uses `ECDH-ES+A256KW` with a fresh ephemeral X25519 key, and encrypts the
/// plaintext with `XC20P`.
///
/// Envelope: `anoncrypt(plaintext)`
/// Media Type: `application/didcomm-encrypted+json`
/// See: https://identity.foundation/didcomm-messaging/spec/#anonymous-encryption
pub fn didcomm_anoncrypt_to_public_key(
	to_public_key: Vec<u8>,
	header: DidCommHeader,
	body: Option<&str>,
) -> Result<String, SignError> {
	let (recipient_kid, plaintext) = build_anoncrypt_plaintext(header, body)?;
	didcomm_anoncrypt_envelope::seal(plaintext.as_bytes(), &recipient_kid, &to_public_key)
		.map_err(SignError::InvalidArgument)
}

/// Compatibility wrapper for callers that still pass a sender private key.
///
/// The sender private key is intentionally ignored. True anoncrypt does not use a sender key.
#[deprecated(note = "use didcomm_anoncrypt_to_public_key; anoncrypt does not need a sender private key")]
pub fn didcomm_anoncrypt(
	_from_private_key: Secret,
	to_public_key: Vec<u8>,
	header: DidCommHeader,
	body: Option<&str>,
) -> Result<String, SignError> {
	didcomm_anoncrypt_to_public_key(to_public_key, header, body)
}

fn build_anoncrypt_plaintext(mut header: DidCommHeader, body: Option<&str>) -> Result<(String, String), SignError> {
	if header.to.len() != 1 {
		return Err(SignError::InvalidArgument(anyhow!(
			"anoncrypt currently requires exactly one recipient in header.to"
		)));
	}
	let recipient_kid = header.to.iter().next().cloned().expect("header.to length checked");
	header.from = None;
	let fields = take(&mut header.fields);

	let mut message = Message::new().didcomm_header(into_didcomm_rs_header(header));
	if let Some(body) = body {
		message = message.body(body).map_err(|e| SignError::InvalidArgument(e.into()))?;
	}
	for (key, value) in fields {
		ensure_application_header_field(&key)?;
		message = message.add_header_field(key, value);
	}

	let plaintext = message.as_raw_json().map_err(|e| SignError::InvalidArgument(e.into()))?;
	let plaintext = clean_anoncrypt_plaintext(&plaintext)?;
	Ok((recipient_kid, plaintext))
}

fn ensure_application_header_field(key: &str) -> Result<(), SignError> {
	match key {
		"id" | "type" | "to" | "from" | "thid" | "pthid" | "created_time" | "expires_time" => {
			Err(SignError::InvalidArgument(anyhow!("Reserved key: {}", key)))
		},
		_ => Ok(()),
	}
}

fn clean_anoncrypt_plaintext(plaintext: &str) -> Result<String, SignError> {
	let mut plaintext =
		serde_json::from_str::<serde_json::Value>(plaintext).map_err(|e| SignError::InvalidArgument(e.into()))?;
	let plaintext = plaintext.as_object_mut().expect("DIDComm message serialized from object");
	if plaintext.get("from").is_some_and(serde_json::Value::is_null) {
		plaintext.remove("from");
	}
	serde_json::to_string(&plaintext).map_err(|e| SignError::InvalidArgument(e.into()))
}

pub fn didcomm_anoncrypt_receive(
	to_private_key: Secret,
	incoming: &str,
) -> Result<(DidCommHeader, Option<String>), ReceiveError> {
	let plaintext =
		didcomm_anoncrypt_envelope::open(incoming, to_private_key.divulge()).map_err(ReceiveError::Decrypt)?;
	let message: Message = serde_json::from_slice(&plaintext).map_err(|e| ReceiveError::UnknownFormat(e.into()))?;

	let mut header = from_didcomm_rs_header(message.get_didcomm_header().clone());
	for (key, value) in message.get_application_params() {
		header.fields.insert(key.to_owned(), value.to_owned());
	}

	Ok((header, message.get_body().ok()))
}

#[cfg(test)]
mod tests {
	use super::{didcomm_anoncrypt_receive, didcomm_anoncrypt_to_public_key};
	use crate::{DidCommHeader, DidKeyIdentity, Identity};
	use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
	use serde_json::Value;

	fn protected_header(message: &str) -> Value {
		let envelope: Value = serde_json::from_str(message).unwrap();
		let protected = envelope
			.get("protected")
			.and_then(Value::as_str)
			.expect("JWE must contain a protected header");
		let decoded = URL_SAFE_NO_PAD.decode(protected).unwrap();
		serde_json::from_slice(&decoded).unwrap()
	}

	#[test]
	fn anoncrypt_uses_ecdh_es_without_sender_or_signature_headers() {
		let from = DidKeyIdentity::generate(Some(&[10; 32]));
		let to = DidKeyIdentity::generate_x25519(Some(&[11; 32]));

		let header = DidCommHeader {
			id: "spec-anoncrypt".to_owned(),
			from: Some(from.identity().to_owned()),
			to: vec![to.identity().to_owned()].into_iter().collect(),
			message_type: "hello".to_owned(),
			..Default::default()
		};
		let message = didcomm_anoncrypt_to_public_key(to.public_key_bytes(), header, Some("\"body\"")).unwrap();
		let protected = protected_header(&message);

		assert_eq!(protected.get("typ").and_then(Value::as_str), Some("application/didcomm-encrypted+json"));
		assert_eq!(protected.get("alg").and_then(Value::as_str), Some("ECDH-ES+A256KW"));
		assert_eq!(protected.get("enc").and_then(Value::as_str), Some("XC20P"));
		assert!(protected.get("epk").is_some(), "anoncrypt must publish an ephemeral ECDH key");
		assert!(protected.get("apv").is_some(), "anoncrypt must bind the recipient kid list");
		assert!(protected.get("skid").is_none(), "anoncrypt must not publish a sender key id");
		assert!(protected.get("apu").is_none(), "anoncrypt must not publish producer info");
		assert!(!message.contains(from.identity()), "wire envelope must not contain the real sender DID");
		assert!(!message.contains("\"signatures\""), "anoncrypt(plaintext) must not add a JWS layer");
		assert!(!message.contains("\"signature\""), "anoncrypt(plaintext) must not add a JWS layer");

		let (received_header, _) = didcomm_anoncrypt_receive(to.private_key_bytes(), &message).unwrap();
		assert_eq!(received_header.from, None, "anoncrypt plaintext must not carry a sender DID");
	}

	#[test]
	fn custom_header_fields_round_trip() {
		let from = DidKeyIdentity::generate(Some(&[12; 32]));
		let to = DidKeyIdentity::generate_x25519(Some(&[13; 32]));

		let header = DidCommHeader {
			id: "custom-fields".to_owned(),
			from: Some(from.identity().to_owned()),
			to: vec![to.identity().to_owned()].into_iter().collect(),
			message_type: "hello".to_owned(),
			fields: vec![("goal_code".to_owned(), "com.example.test".to_owned())]
				.into_iter()
				.collect(),
			..Default::default()
		};
		let message = didcomm_anoncrypt_to_public_key(to.public_key_bytes(), header, Some("\"body\"")).unwrap();

		let (received_header, received_body) = didcomm_anoncrypt_receive(to.private_key_bytes(), &message).unwrap();
		assert_eq!(received_body.as_deref(), Some("\"body\""));
		assert_eq!(received_header.fields.get("goal_code").map(String::as_str), Some("com.example.test"));
	}

	#[test]
	fn smoke() {
		let from = DidKeyIdentity::generate_x25519(Some(&[1; 32]));
		let to = DidKeyIdentity::generate_x25519(Some(&[2; 32]));
		let other = DidKeyIdentity::generate_x25519(Some(&[3; 32]));
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
		let message = didcomm_anoncrypt_to_public_key(to.public_key_bytes(), header, None).unwrap();
		println!("message({}): {}", message.len(), message); // 2462

		// receive
		let (receviced_header, receviced_body) = didcomm_anoncrypt_receive(to.private_key_bytes(), &message).unwrap();
		assert_eq!(Some("{}".to_owned()), receviced_body);
		assert_eq!("test", receviced_header.id);

		// receive other
		let received_other = didcomm_anoncrypt_receive(other.private_key_bytes(), &message);
		assert!(received_other.is_err());
	}

	#[test]
	#[allow(deprecated)]
	fn deprecated_sender_key_wrapper_delegates_to_sender_free_anoncrypt() {
		use super::didcomm_anoncrypt;

		let from = DidKeyIdentity::generate(Some(&[41; 32]));
		let to = DidKeyIdentity::generate_x25519(Some(&[42; 32]));
		let header = DidCommHeader {
			id: "compat-wrapper".to_owned(),
			from: Some(from.identity().to_owned()),
			to: vec![to.identity().to_owned()].into_iter().collect(),
			message_type: "hello".to_owned(),
			..Default::default()
		};

		let message =
			didcomm_anoncrypt(from.private_key_bytes(), to.public_key_bytes(), header, Some("\"ok\"")).unwrap();
		let (received_header, body) = didcomm_anoncrypt_receive(to.private_key_bytes(), &message).unwrap();

		assert_eq!(received_header.id, "compat-wrapper");
		assert_eq!(body.as_deref(), Some("\"ok\""));
	}
}
