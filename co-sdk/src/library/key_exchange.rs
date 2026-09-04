// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use co_identity::{DidCommHeader, Identity, PrivateIdentity};
use co_network::{EncodedMessage, PeerId};
use co_primitives::{to_json_string, CoDateRef, CoId};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const CO_DIDCOMM_KEY_REQUEST: &str = "co-key-request";
pub const CO_DIDCOMM_KEY_RESPONSE: &str = "co-key-response";

/// Create an signed key request message.
/// As we may send this request to any CO participant it's only signed by the sender and without an explicit recipent.
pub fn create_key_request_message<F>(
	date: &CoDateRef,
	from: &F,
	payload: KeyRequestPayload,
	expire: Duration,
) -> anyhow::Result<(DidCommHeader, EncodedMessage)>
where
	F: PrivateIdentity + Send + Sync + 'static,
{
	let (from_didcomm, mut header) = DidCommHeader::create_from(date, from, CO_DIDCOMM_KEY_REQUEST)?;
	header.expires_time = Some((date.now_duration() + expire).as_secs());
	let body = to_json_string(&payload)?;
	let message = from_didcomm.jws(header.clone(), &body)?;
	Ok((header, EncodedMessage(message.into_bytes())))
}

/// Create an encrypted key response message.
pub fn create_key_response_message<F, T>(
	date: &CoDateRef,
	from: &F,
	to: &T,
	request_message_id: String,
	payload: KeyResponsePayload,
) -> anyhow::Result<(DidCommHeader, EncodedMessage)>
where
	F: PrivateIdentity + Send + Sync + 'static,
	T: Identity + Send + Sync + 'static,
{
	let (from_didcomm, to_didcomm, mut header) = DidCommHeader::create(date, from, to, CO_DIDCOMM_KEY_RESPONSE)?;
	header.thid = Some(request_message_id);
	let body = to_json_string(&payload)?;
	let message = from_didcomm.jwe(&to_didcomm, header.clone(), &body)?;
	Ok((header, EncodedMessage(message.into_bytes())))
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct KeyRequestPayload {
	/// The requesters PeerId.
	/// When signed this creates an relation between the DID and the PeerID to enable receiver trust.
	/// This is to mitigate forwarding attacks because we don't send an to header.
	pub peer: PeerId,

	/// The ID of the CO.
	pub id: CoId,

	/// The requested key uri. If None the current key is returned.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum KeyResponsePayload {
	Ok(co_core_keystore::Key),
	Failure,
}

#[cfg(test)]
mod tests {
	use super::{create_key_response_message, KeyResponsePayload};
	use crate::library::key_exchange::KeyRequestPayload;
	use co_core_keystore::Key;
	use co_identity::{
		DidKeyIdentity, DidKeyIdentityResolver, Identity, MemoryPrivateIdentityResolver, Message, PrivateIdentityBox,
	};
	use co_network::PeerId;
	use co_primitives::{from_json, tags, to_json, CoId, Secret, StaticCoDate};
	use serde_json::Value;

	const SHARED_KEY: &str = "top-secret-shared-key";

	fn shared_key_response() -> KeyResponsePayload {
		KeyResponsePayload::Ok(Key {
			description: "test".to_owned(),
			name: "test".to_owned(),
			tags: tags!("hello": "world"),
			uri: "urn:test".to_owned(),
			secret: co_core_keystore::Secret::SharedKey(Secret::new(SHARED_KEY.as_bytes().to_vec())),
		})
	}

	#[tokio::test]
	async fn key_response_is_encrypted_and_reaches_only_its_recipient() {
		let from = DidKeyIdentity::generate_x25519(Some(&[51; 32]));
		let to = DidKeyIdentity::generate_x25519(Some(&[52; 32]));

		let (_, message) =
			create_key_response_message(&StaticCoDate(0), &from, &to, "request".to_owned(), shared_key_response())
				.unwrap();
		assert!(!String::from_utf8_lossy(&message.0).contains(SHARED_KEY), "the envelope carries the key in plaintext");

		let received = Message::receive(
			DidKeyIdentityResolver::new(),
			MemoryPrivateIdentityResolver::from([PrivateIdentityBox::new(to.clone())]),
			&message.0,
		)
		.await
		.unwrap();
		assert_eq!(received.body_deserialize::<KeyResponsePayload>().unwrap(), shared_key_response());
	}

	#[tokio::test]
	async fn captured_key_response_entry_does_not_open_another_key_response() {
		let from = DidKeyIdentity::generate_x25519(Some(&[56; 32]));
		let to = DidKeyIdentity::generate_x25519(Some(&[57; 32]));
		let response =
			|date| create_key_response_message(date, &from, &to, "request".to_owned(), shared_key_response());

		let captured: Value = serde_json::from_slice(&response(&StaticCoDate(0)).unwrap().1 .0).unwrap();
		let mut target: Value = serde_json::from_slice(&response(&StaticCoDate(0)).unwrap().1 .0).unwrap();
		target["header"] = captured["header"].clone();
		target["encrypted_key"] = captured["encrypted_key"].clone();
		let forged = serde_json::to_vec(&target).unwrap();

		let received = Message::receive(
			DidKeyIdentityResolver::new(),
			MemoryPrivateIdentityResolver::from([PrivateIdentityBox::new(to.clone())]),
			&forged,
		)
		.await;
		assert!(received.is_err(), "a captured recipient entry opened another key response");
	}

	#[tokio::test]
	async fn key_response_names_a_validated_sender() {
		let from = DidKeyIdentity::generate_x25519(Some(&[54; 32]));
		let to = DidKeyIdentity::generate_x25519(Some(&[55; 32]));

		let (_, message) =
			create_key_response_message(&StaticCoDate(0), &from, &to, "request".to_owned(), shared_key_response())
				.unwrap();

		let received = Message::receive(
			DidKeyIdentityResolver::new(),
			MemoryPrivateIdentityResolver::from([PrivateIdentityBox::new(to.clone())]),
			&message.0,
		)
		.await
		.unwrap();

		assert!(received.is_validated_sender(), "the key response has no validated sender");
		assert_eq!(received.sender().map(String::as_str), Some(from.identity()));
	}

	#[test]
	fn test_serialize_request() {
		let payload = KeyRequestPayload { peer: PeerId::random(), id: CoId::new("test"), key: None };
		let json = to_json(&payload).unwrap();
		let deserialized: KeyRequestPayload = from_json(&json).unwrap();
		assert_eq!(deserialized, payload);
	}

	#[test]
	fn test_serialize_response_json_payload() {
		let payload = KeyResponsePayload::Ok(Key {
			description: "test".to_owned(),
			name: "test".to_owned(),
			tags: tags!("hello": "world"),
			uri: "urn:test".to_owned(),
			secret: co_core_keystore::Secret::SharedKey(Secret::new("test".as_bytes().to_vec())),
		});
		let json = to_json(&payload).unwrap();
		let deserialized: KeyResponsePayload = from_json(&json).unwrap();
		assert_eq!(deserialized, payload);
	}
}
