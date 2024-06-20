use anyhow::anyhow;
use co_identity::{DidCommHeader, PrivateIdentity};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::value::RawValue;

/// DIDComm Message
///
/// See: https://identity.foundation/didcomm-messaging/spec/v2.1/#iana-media-types
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedMessage(pub Vec<u8>);
impl EncodedMessage {
	/// Create plaintext JSON message.
	pub fn create_plain_json<T: Serialize>(header: DidCommHeader, body: &T) -> Result<Self, anyhow::Error> {
		Ok(Self(serde_ipld_dagjson::to_vec(&DidCommMessage { header, body })?))
	}

	/// Create signed JSON message.
	pub fn create_signed_json<T, P>(identity: &P, header: DidCommHeader, body: &T) -> Result<Self, anyhow::Error>
	where
		T: Serialize,
		P: PrivateIdentity + Send + Sync + 'static,
	{
		let context = identity.didcomm_private().ok_or(anyhow!("No didcomm context"))?;
		let body_json = serde_ipld_dagjson::to_vec(body)?;
		let jws = context.jws(header, std::str::from_utf8(&body_json)?)?;
		Ok(Self(jws.into_bytes()))
	}

	/// Sign message. Assuming we currently hold a plain text JSON message.
	pub fn sign<P>(self, identity: &P) -> Result<Self, anyhow::Error>
	where
		P: PrivateIdentity + Send + Sync + 'static,
	{
		let message: DidCommMessage<&RawValue> = serde_ipld_dagjson::from_slice(&self.0)?;
		Self::create_signed_json(identity, message.header, &message.body)
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
			return Ok(serde_ipld_dagjson::from_slice(data.as_bytes())?);
		}
		if let Some(data) = self.cbor() {
			return Ok(serde_ipld_dagcbor::from_slice(data)?);
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

#[derive(Debug, Serialize, Deserialize)]
struct DidCommMessage<T> {
	#[serde(flatten)]
	header: DidCommHeader,
	body: T,
}

#[cfg(test)]
mod tests {
	use super::EncodedMessage;
	use serde::Serialize;

	#[derive(Debug, Serialize)]
	struct Test {
		count: i32,
	}

	#[test]
	fn test_json() {
		let data = Test { count: 10 };
		let json = serde_json::to_string(&data).unwrap();
		let message: EncodedMessage = json.clone().into();
		assert_eq!(Some(json.as_str()), message.json());
	}

	#[test]
	fn test_cbor() {
		let data = Test { count: 10 };
		let cbor = serde_ipld_dagcbor::to_vec(&data).unwrap();
		let message: EncodedMessage = cbor.clone().into();
		// println!("f: {}", cbor[0] as u8);
		assert_eq!(Some(&cbor[..]), message.cbor());
	}
}
