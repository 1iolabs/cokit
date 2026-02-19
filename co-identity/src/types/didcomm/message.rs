// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{
	library::didcomm_receive::didcomm_receive, DidCommContext, DidCommHeader, Identity, IdentityResolver,
	PrivateIdentity, PrivateIdentityResolver, ReceiveError,
};
use anyhow::anyhow;
use co_primitives::{from_json_string, Did};
use didcomm_rs::{Jwe, Jws, MessageType};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::value::RawValue;

/// DIDComm Message Envelope
///
/// See: https://identity.foundation/didcomm-messaging/spec/v2.1/#iana-media-types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
	/// Unsiged JSON encoded message.
	///
	/// Envelope: `plaintext` (no envelope)
	/// Media Type: `application/didcomm-plain+json`
	PlainJson { header: DidCommHeader, body: String },

	/// Signed JSON encoded message.
	///
	/// The identity has been verified when this is constructed.
	///
	/// Envelope: `signed(plaintext)`
	/// Media Type: `application/didcomm-signed+json`
	SignedJson { sender: Did, header: DidCommHeader, body: String },

	/// Encrypted JSON encoded message.
	///
	/// Guarantees confidentiality and integrity without revealing the identity of the sender.
	///
	/// Envelope: `anoncrypt(plaintext)`
	/// Media Type: `application/didcomm-encrypted+json`
	AnonCryptJson { header: DidCommHeader, body: String },

	/// Encrypted authenticated JSON encoded message.
	///
	/// Guarantees confidentiality and integrity. Also proves the identity of the sender – but in a way that only the
	/// recipient can verify. This is the default wrapping choice, and SHOULD be used unless a different goal is
	/// clearly identified. By design, this combination and all other combinations that use encryption in their
	/// outermost layer share an identical IANA media type, because only the recipient should care about the
	/// difference. Media Type: `application/didcomm-encrypted+json`
	///
	/// The identity has been verified when this is constructed.
	///
	/// Envelope: `authcrypt(plaintext)`
	/// Media Type: `application/didcomm-encrypted+json`
	AuthCryptJson { sender: Did, header: DidCommHeader, body: String },
}
impl Message {
	/// Receive message from data.
	pub async fn receive<I, P>(sender_resolver: I, recipent_resolver: P, data: &[u8]) -> Result<Message, ReceiveError>
	where
		I: IdentityResolver + Send + Sync + 'static,
		P: PrivateIdentityResolver + Send + Sync + 'static,
	{
		let message = std::str::from_utf8(data).map_err(|e| ReceiveError::UnknownFormat(e.into()))?;
		let message_type = get_message_type(message).map_err(ReceiveError::UnknownFormat)?;
		if message_type == MessageType::DidCommJwe {
			let jwe: Jwe = serde_json::from_str(message).map_err(|e| ReceiveError::UnknownFormat(e.into()))?;

			// for anoncrypt this is usually the ephemeral sender did
			let sender_identity = if let Some(sender_kid) = &jwe.get_skid() {
				Some(
					sender_resolver
						.resolve(sender_kid)
						.await
						.map_err(|e| ReceiveError::BadDid(sender_kid.to_owned(), e.into()))?,
				)
			} else {
				None
			};

			// get recipents
			let recipents = jwe
				.recipients
				.unwrap_or_else(|| jwe.recipient.map(|item| vec![item]).unwrap_or_default());
			let recipent_resolver_ref = &recipent_resolver;

			// try to receive message
			for recipent in &recipents {
				let recipent_did = match &recipent.header.kid {
					Some(kid) => kid,
					None => continue,
				};
				let recipent_identity = match recipent_resolver_ref.resolve_private(recipent_did).await {
					Ok(i) => i,
					Err(_) => continue,
				};
				let recipent_didcomm_context = match recipent_identity.didcomm_private() {
					Some(i) => i,
					None => continue,
				};
				let (header, body) = recipent_didcomm_context.receive(&sender_resolver, message).await?;

				// result
				// when the encryption sender key is equal to the from header we have authcrypt
				// See: https://identity.foundation/didcomm-messaging/spec/v2.1/#message-headers
				if let Some(from) = &header.from {
					if let Some(sender_identity) = sender_identity {
						if from == sender_identity.identity() {
							return Ok(Message::AuthCryptJson {
								sender: sender_identity.identity().to_owned(),
								header,
								body,
							});
						}
					}
				}
				return Ok(Message::AnonCryptJson { header, body });
			}
			return Err(ReceiveError::NoRecipent);
		}
		if message_type == MessageType::DidCommJws {
			let (header, body) = didcomm_receive(None, &sender_resolver, message).await?;

			// resolve sender
			let sender = verify_signing_identity(&sender_resolver, &header, message).await?;

			// result
			return Ok(Message::SignedJson { sender, header, body });
		}
		if message_type == MessageType::DidCommRaw {
			let plain_message: DidCommMessage =
				serde_json::from_str(message).map_err(|e| ReceiveError::UnknownFormat(e.into()))?;
			return Ok(Message::PlainJson {
				header: plain_message.header,
				body: plain_message.body.map(|r| r.get()).unwrap_or("null").to_owned(),
			});
		}
		Err(ReceiveError::UnknownFormat(anyhow!("Expected JSON as JWE, JWS or plain DIDComm")))
	}

	/// Return message header.
	pub fn header(&self) -> &DidCommHeader {
		match self {
			Message::PlainJson { header, body: _ } => header,
			Message::SignedJson { sender: _, header, body: _ } => header,
			Message::AnonCryptJson { header, body: _ } => header,
			Message::AuthCryptJson { sender: _, header, body: _ } => header,
		}
	}

	/// Return Body as JSON string.
	pub fn body(&self) -> &str {
		match self {
			Message::PlainJson { header: _, body } => body,
			Message::SignedJson { sender: _, header: _, body } => body,
			Message::AnonCryptJson { header: _, body } => body,
			Message::AuthCryptJson { sender: _, header: _, body } => body,
		}
	}

	/// Try to deserialize message to T.
	pub fn body_deserialize<T: DeserializeOwned>(&self) -> Result<T, anyhow::Error> {
		Ok(from_json_string(self.body())?)
	}

	/// Test if message is validated.
	pub fn is_validated_sender(&self) -> bool {
		self.sender().is_some()
	}

	/// Get validated sender.
	pub fn sender(&self) -> Option<&Did> {
		match self {
			Message::AuthCryptJson { sender, header, body: _ } if Some(sender) == header.from.as_ref() => Some(sender),
			Message::SignedJson { sender, header, body: _ } if Some(sender) == header.from.as_ref() => Some(sender),
			_ => None,
		}
	}

	// Convert into inner.
	pub fn into_inner(self) -> (DidCommHeader, String) {
		match self {
			Message::PlainJson { header, body } => (header, body),
			Message::SignedJson { sender: _, header, body } => (header, body),
			Message::AnonCryptJson { header, body } => (header, body),
			Message::AuthCryptJson { sender: _, header, body } => (header, body),
		}
	}
}

/// Helper type to check if received message is plain, signed or encrypted
///
/// Source: https://github.com/dkuhnert/didcomm-rs/blob/main/src/messages/helpers/receive.rs
#[derive(Serialize, Deserialize, Debug)]
struct UnknownReceivedMessage<'a> {
	#[serde(borrow)]
	pub signature: Option<&'a RawValue>,

	#[serde(borrow)]
	pub signatures: Option<&'a RawValue>,

	#[serde(borrow)]
	pub iv: Option<&'a RawValue>,
}

/// Tries to parse message and checks for well known fields to derive message type.
///
/// Source: https://github.com/dkuhnert/didcomm-rs/blob/main/src/messages/helpers/receive.rs
fn get_message_type(message: &str) -> Result<MessageType, anyhow::Error> {
	// try to skip parsing by using known fields from jwe/jws
	let to_check: UnknownReceivedMessage = serde_json::from_str(message)?;
	if to_check.iv.is_some() {
		return Ok(MessageType::DidCommJwe);
	}
	if to_check.signatures.is_some() || to_check.signature.is_some() {
		return Ok(MessageType::DidCommJws);
	}
	let didcomm_message: Option<didcomm_rs::Message> = serde_json::from_str(message).ok();
	if let Some(didcomm_message) = didcomm_message {
		return Ok(didcomm_message.get_jwm_header().typ.clone());
	}
	let _plain_message: DidCommMessage = serde_json::from_str(message)?;
	Ok(MessageType::DidCommRaw)
}

/// Verify plain text `from` header matches any signature `kid`.
/// We accept if:
/// - the `kid` is a known from public key.
/// - the `kid` is the same value as the `from` header.
async fn verify_signing_identity<I>(
	sender_resolver: &I,
	header: &DidCommHeader,
	message: &str,
) -> Result<Did, ReceiveError>
where
	I: IdentityResolver + Send + Sync + 'static,
{
	if let Some(from) = &header.from {
		// from
		let sender_identity = sender_resolver
			.resolve(from)
			.await
			.map_err(|e| ReceiveError::BadDid(from.to_owned(), e.into()))?;
		let sender_context = sender_identity
			.didcomm_public()
			.ok_or_else(|| ReceiveError::BadDid(from.to_owned(), anyhow!("No didcomm public context.")))?;
		let sender_public_key = sender_context
			.verification_method()
			.public_key_bytes()
			.map_err(|err| ReceiveError::BadDid(from.to_owned(), err))?;
		let sender_kid = hex::encode(&sender_public_key);

		// check signature kid
		let jws: Jws = serde_json::from_str(message).map_err(|e| ReceiveError::UnknownFormat(e.into()))?;
		let signatures = if let Some(signatures) = jws.signatures {
			signatures
		} else if let Some(signature) = jws.signature {
			vec![signature]
		} else {
			vec![]
		};
		for signature in signatures {
			if let Some(kid) = &signature.get_kid() {
				if kid == &sender_kid || kid == sender_identity.identity() {
					return Ok(from.clone());
				}
			}
		}
		Err(ReceiveError::InvalidSigningKeyId(anyhow!("Can not match from header")))
	} else {
		Err(ReceiveError::UnknownFormat(anyhow!("No from header")))
	}
}

#[derive(Debug, Serialize, Deserialize)]
struct DidCommMessage<'a> {
	#[serde(flatten)]
	header: DidCommHeader,
	#[serde(borrow)]
	body: Option<&'a RawValue>,
}
