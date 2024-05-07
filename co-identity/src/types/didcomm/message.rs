use crate::DidCommHeader;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Message<T> {
	#[serde(flatten)]
	pub header: DidCommHeader,

	/// OPTIONAL. The body attribute contains all the data and structure that are uniquely defined for the schema
	/// associated with the type attribute. If present, it MUST be a JSON object that conforms to RFC 7159.
	pub body: T,
	// /// OPTIONAL. See Attachments for detail.
	// pub attachments: Vec<A>;
}
impl<T> Message<T> {
	/// Encode this message as dag-cbor.
	pub fn json(&self) -> Result<String, anyhow::Error>
	where
		T: Serialize,
	{
		Ok(serde_json::to_string(&self)?)
	}

	/// Decode dag-cbor data to message.
	pub fn from_json(data: &str) -> Result<Self, anyhow::Error>
	where
		T: DeserializeOwned,
	{
		Ok(serde_json::from_str(&data)?)
	}

	/// Encode this message as dag-cbor.
	pub fn cbor(&self) -> Result<Vec<u8>, anyhow::Error>
	where
		T: Serialize,
	{
		Ok(serde_ipld_dagcbor::to_vec(&self)?)
	}

	/// Decode dag-cbor data to message.
	pub fn from_cbor(data: &[u8]) -> Result<Self, anyhow::Error>
	where
		T: DeserializeOwned,
	{
		Ok(serde_ipld_dagcbor::from_slice(&data)?)
	}
}

/// Message without body.
pub type MetadataMessage = Message<Option<()>>;
