use co_identity::{PrivateIdentity, SignError};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::BTreeSet;

/// See: https://identity.foundation/didcomm-messaging/spec/#message-headers
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Message<T> {
	/// REQUIRED. Message ID. The id attribute value MUST be unique to the sender, across all messages they send. See
	/// Threading > Message IDs for constraints on this value.
	pub id: String,

	/// REQUIRED. A URI that associates the body of a plaintext message with a published and versioned schema. Useful
	/// for message handling in application-level protocols. The type attribute value MUST be a valid message type URI,
	/// that when resolved gives human readable information about the message category.
	#[serde(rename = "type")]
	pub message_type: String,

	/// OPTIONAL. Identifier(s) for recipients. MUST be an array of strings where each element is a valid DID or DID
	/// URL (without the fragment component) that identifies a member of the message's intended audience. These values
	/// are useful for recipients to know which of their keys can be used for decryption. It is not possible for one
	/// recipient to verify that the message was sent to a different recipient.
	///
	/// When Alice sends the same plaintext message to Bob and Carol, it is by inspecting this header that the
	/// recipients learn the message was sent to both of them. If the header is omitted, each recipient SHOULD assume
	/// they are the only recipient (much like an email sent only to BCC: addresses).
	///
	/// For signed messages, there are specific requirements around properly defining the to header outlined in the
	/// DIDComm Signed Message definition above. This prevents certain kind of forwarding attacks, where a message that
	/// was not meant for a given recipient is forwarded along with its signature to a recipient which then might
	/// blindly trust it because of the signature.
	///
	/// Upon reception of a message with a defined to header, the recipient SHOULD verify that their own identifier
	/// appears in the list. Implementations MUST NOT fail to accept a message when this is not the case, but SHOULD
	/// give a warning to their user as it could indicate malicious intent from the sender.
	///
	/// The to header cannot be used for routing, since it is encrypted at every intermediate point in a route.
	/// Instead, the forward message contains a next attribute in its body that specifies the target for the next
	/// routing operation.
	#[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
	pub to: BTreeSet<String>,

	/// OPTIONAL when the message is to be encrypted via anoncrypt; REQUIRED when the message is encrypted via
	/// authcrypt. Sender identifier. The from attribute MUST be a string that is a valid DID or DID URL (without the
	/// fragment component) which identifies the sender of the message. When a message is encrypted, the sender key
	/// MUST be authorized for encryption by this DID. Authorization of the encryption key for this DID MUST be
	/// verified by message recipient with the proper proof purposes. When the sender wishes to be anonymous using
	/// authcrypt, it is recommended to use a new DID created for the purpose to avoid correlation with any other
	/// behavior or identity. Peer DIDs are lightweight and require no ledger writes, and therefore a good method to
	/// use for this purpose.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub from: Option<String>,

	/// OPTIONAL. Thread identifier. Uniquely identifies the thread that the message belongs to. If not included, the
	/// id property of the message MUST be treated as the value of the thid. See Threads for details.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub thid: Option<String>,

	/// OPTIONAL. Thread identifier. Uniquely identifies the thread that the message belongs to. If not included, the
	/// id property of the message MUST be treated as the value of the thid. See Threads for details.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub pthid: Option<String>,

	/// OPTIONAL but recommended. Message Created Time. This attribute is used for the sender to express when they
	/// created the message, expressed in UTC Epoch Seconds (seconds since 1970-01-01T00:00:00Z) as an integer. This
	/// allows the recipient to guess about transport latency and clock divergence. The difference between when a
	/// message is created and when it is sent is assumed to be negligible; this lets timeout logic start from this
	/// value.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub created_time: Option<u64>,

	/// OPTIONAL. Message Expires Time. This attribute is used for the sender to express when they will consider the
	/// message to be expired, expressed in UTC Epoch Seconds (seconds since 1970-01-01T00:00:00Z) as an integer. By
	/// default, the meaning of “expired” is that the sender will abort the protocol if it doesn’t get a response by
	/// this time. However, protocols can nuance this in their formal spec. For example, an online auction protocol
	/// might specify that timed out bids must be ignored instead of triggering a cancellation of the whole auction.
	/// When omitted from any given message, the message is considered to have no expiration by the sender.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub expires_time: Option<u64>,

	/// OPTIONAL. The body attribute contains all the data and structure that are uniquely defined for the schema
	/// associated with the type attribute. If present, it MUST be a JSON object that conforms to RFC 7159.
	pub body: T,
}
impl<T> Message<T> {
	/// Sign this message as `application/didcomm-signed+cbor`.
	pub fn sign<I>(&self, _identity: &I) -> Result<Vec<u8>, SignError>
	where
		I: PrivateIdentity + Send + Sync + 'static,
		T: Serialize,
	{
		let data = self.cbor().map_err(|e| SignError::Other(e.into()))?;
		//let signature = identity.sign(&data)?;
		// TODO: use JWS instead of append
		Ok(data)
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
