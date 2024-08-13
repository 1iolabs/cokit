use anyhow::anyhow;
use co_identity::{DidCommHeader, Identity, PrivateIdentity};
use co_primitives::{from_cbor, from_json, from_json_string, to_json, to_json_string};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::value::RawValue;
use std::fmt::Debug;

/// DIDComm Message
///
/// See: https://identity.foundation/didcomm-messaging/spec/v2.1/#iana-media-types
#[derive(Clone, PartialEq, Eq)]
pub struct EncodedMessage(pub Vec<u8>);
impl EncodedMessage {
	/// Create plaintext JSON message.
	pub fn create_plain_json<T: Serialize>(header: DidCommHeader, body: &T) -> Result<(String, Self), anyhow::Error> {
		let message_id = header.id.clone();
		Ok((message_id, Self(to_json(&DidCommMessage { header, body })?)))
	}

	/// Create signed JSON message.
	///
	/// Note: This will overwrite from header.
	pub fn create_signed_json<T, P>(
		from: &P,
		mut header: DidCommHeader,
		body: &T,
	) -> Result<(String, Self), anyhow::Error>
	where
		T: Serialize,
		P: PrivateIdentity + Send + Sync + 'static,
	{
		let from_didcomm = from
			.didcomm_private()
			.ok_or(anyhow::anyhow!("unsupported identity: from: no private didcomm context"))?;
		header.from = Some(from.identity().to_owned());
		let message_id = header.id.clone();
		let body_json = to_json_string(body)?;
		let envelope = from_didcomm.jws(header, &body_json)?;
		Ok((message_id, Self(envelope.into_bytes())))
	}

	/// Create encrypted JSON message.
	///
	/// Note: This will overwrite from and to header.
	pub fn create_encrypted_json<T, P, I>(
		from: &P,
		to: &I,
		mut header: DidCommHeader,
		body: &T,
	) -> Result<(String, Self), anyhow::Error>
	where
		T: Serialize,
		P: PrivateIdentity + Send + Sync + 'static,
		I: Identity + Send + Sync + 'static,
	{
		let from_didcomm = from
			.didcomm_private()
			.ok_or(anyhow::anyhow!("unsupported identity: from: no private didcomm context"))?;
		let to_didcomm = to
			.didcomm_public()
			.ok_or(anyhow::anyhow!("unsupported identity: to: no public didcomm context"))?;
		header.from = Some(from.identity().to_owned());
		header.to = [to.identity().to_owned()].into_iter().collect();
		let message_id = header.id.clone();
		let body_json = to_json_string(body)?;
		let envelope = from_didcomm.jwe(&to_didcomm, header, &body_json)?;
		Ok((message_id, Self(envelope.into_bytes())))
	}

	/// Sign message. Assuming we currently hold a plain text JSON message.
	pub fn sign<P>(self, from: &P) -> Result<Self, anyhow::Error>
	where
		P: PrivateIdentity + Send + Sync + 'static,
	{
		let message: DidCommMessage<&RawValue> = from_json(&self.0)?;
		Ok(Self::create_signed_json(from, message.header, &message.body)?.1)
	}

	/// Encrypt message. Assuming we currently hold a plain text JSON message.
	pub fn encrypt<P, I>(self, from: &P, to: &I) -> Result<Self, anyhow::Error>
	where
		P: PrivateIdentity + Send + Sync + 'static,
		I: Identity + Send + Sync + 'static,
	{
		let message: DidCommMessage<&RawValue> = from_json(&self.0)?;
		Ok(Self::create_encrypted_json(from, to, message.header, &message.body)?.1)
	}

	/// Get message as JSON string. Returning None if not JSON Object.
	///
	/// Note: No verification will be done.
	pub fn json(&self) -> Option<&str> {
		let data = &self.0;
		if !data.is_empty() && data[0] == '{' as u8 {
			match std::str::from_utf8(&data) {
				Ok(str) => Some(str),
				Err(_) => None,
			}
		} else {
			None
		}
	}

	/// Get message as CBOR. Returning None if not CBOR Map.
	///
	/// Note: No verification will be done.
	/// See: https://www.rfc-editor.org/rfc/rfc8949.html#section-3.1
	pub fn cbor(&self) -> Option<&[u8]> {
		let data = &self.0;
		// check first 3 bytes are 101 = 5
		if !data.is_empty() && (data[0] & 7u8 << 5) == (5u8 << 5) {
			Some(data)
		} else {
			None
		}
	}

	/// Try to deserialize message to T.
	pub fn deserialize<T: DeserializeOwned>(&self) -> Result<T, anyhow::Error> {
		if let Some(data) = self.json() {
			return Ok(from_json_string(data)?);
		}
		if let Some(data) = self.cbor() {
			return Ok(from_cbor(data)?);
		}
		Err(anyhow!("unknown format"))
	}
}
impl From<Vec<u8>> for EncodedMessage {
	fn from(value: Vec<u8>) -> Self {
		EncodedMessage(value)
	}
}
impl From<String> for EncodedMessage {
	fn from(value: String) -> Self {
		EncodedMessage(value.into())
	}
}
impl From<&str> for EncodedMessage {
	fn from(value: &str) -> Self {
		EncodedMessage(value.into())
	}
}
impl From<EncodedMessage> for Vec<u8> {
	fn from(val: EncodedMessage) -> Self {
		val.0
	}
}
impl AsRef<[u8]> for EncodedMessage {
	fn as_ref(&self) -> &[u8] {
		&self.0
	}
}
impl Debug for EncodedMessage {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		if let Some(json) = self.json() {
			f.debug_tuple("EncodedMessage").field(&json).finish()
		} else {
			f.debug_tuple("EncodedMessage").field(&self.0).finish()
		}
	}
}

#[derive(Debug, Serialize, Deserialize)]
struct DidCommMessage<T> {
	#[serde(flatten)]
	header: DidCommHeader,
	body: T,
}

#[cfg(test)]
mod tests {
	use super::EncodedMessage;
	use co_primitives::{to_cbor, to_json};
	use serde::Serialize;

	#[derive(Debug, Serialize)]
	struct Test {
		count: i32,
	}

	#[test]
	fn test_json() {
		let data = Test { count: 10 };
		let json = to_json(&data).unwrap();
		let message: EncodedMessage = json.into();
		assert_eq!(message.json(), Some("{\"count\":10}"));
	}

	#[test]
	fn test_cbor() {
		let data = Test { count: 10 };
		let cbor = to_cbor(&data).unwrap();
		let message: EncodedMessage = cbor.clone().into();
		// println!("f: {}", cbor[0] as u8);
		assert_eq!(message.cbor(), Some(&cbor[..]));
	}
}
