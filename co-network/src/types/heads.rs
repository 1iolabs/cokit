use co_identity::DidCommHeader;
use co_primitives::{CoId, WeakCid};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum HeadsMessage {
	/// Heads notification.
	#[serde(rename = "h")]
	Heads(CoId, BTreeSet<WeakCid>),

	/// Request heads from peer.
	/// This message must be signed.
	/// Will be responded with one of:
	/// - [`HeadsMessage::Heads`].
	/// - [`HeadsMessage::Error`].
	#[serde(rename = "r")]
	HeadsRequest(CoId),

	/// Error notification.
	#[serde(rename = "e")]
	Error { co: CoId, code: HeadsErrorCode, message: String },
}
impl HeadsMessage {
	/// Message type
	pub fn message_type() -> String {
		"co-heads/1.0".to_string()
	}

	/// DIDComm message header.
	pub fn create_header() -> DidCommHeader {
		let mut header = DidCommHeader::new(Self::message_type());
		header.expires_time = header.created_time.map(|t| t + 120);
		header
	}

	pub fn co(&self) -> &CoId {
		match self {
			HeadsMessage::Heads(co_id, ..) => co_id,
			HeadsMessage::HeadsRequest(co_id) => co_id,
			HeadsMessage::Error { co, .. } => co,
		}
	}
}

#[derive(Debug, Clone, Serialize_repr, Deserialize_repr)]
#[non_exhaustive]
#[repr(u16)]
pub enum HeadsErrorCode {
	Forbidden = 403,
	InternalServerError = 500,
	ServiceUnavailable = 503,
}
