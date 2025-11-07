use co_identity::{DidCommHeader, Identity, PrivateIdentity};
use co_network::EncodedMessage;
use co_primitives::{to_json_string, CoId};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const CO_DIDCOMM_KEY_REQUEST: &str = "co-key-request";
pub const CO_DIDCOMM_KEY_RESPONSE: &str = "co-key-response";

/// Create an signed key request message.
/// As we may send this request to any CO participant it's only signed by the sender and without an explicit recipent.
pub fn create_key_request_message<F>(
	from: &F,
	payload: KeyRequestPayload,
	expire: Duration,
) -> anyhow::Result<(DidCommHeader, EncodedMessage)>
where
	F: PrivateIdentity + Send + Sync + 'static,
{
	let (from_didcomm, mut header) = DidCommHeader::create_from(from, CO_DIDCOMM_KEY_REQUEST)?;
	header.expires_time = Some((SystemTime::now().duration_since(UNIX_EPOCH)? + expire).as_secs());
	let body = to_json_string(&payload)?;
	let message = from_didcomm.jws(header.clone(), &body)?;
	Ok((header, EncodedMessage(message.into_bytes())))
}

/// Create an encrypted key response message.
pub fn create_key_response_message<F, T>(
	from: &F,
	to: &T,
	request_message_id: String,
	payload: KeyResponsePayload,
) -> anyhow::Result<(DidCommHeader, EncodedMessage)>
where
	F: PrivateIdentity + Send + Sync + 'static,
	T: Identity + Send + Sync + 'static,
{
	let (from_didcomm, to_didcomm, mut header) = DidCommHeader::create(from, to, CO_DIDCOMM_KEY_RESPONSE)?;
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
	use super::KeyResponsePayload;
	use crate::library::key_exchange::KeyRequestPayload;
	use co_core_keystore::Key;
	use co_primitives::{from_json, tags, to_json, CoId, Secret};
	use libp2p::PeerId;

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
