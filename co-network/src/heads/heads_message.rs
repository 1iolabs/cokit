use co_identity::DidCommHeader;
use co_primitives::CoId;
use libipld::Cid;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HeadsMessage {
	/// Heads notifictaion.
	#[serde(rename = "h")]
	Heads(CoId, BTreeSet<Cid>),

	/// Request heads from peer.
	/// This message must be signed.
	/// Will be responded with one of:
	/// - [`HeadsMessage::Heads`].
	/// - [`HeadsMessage::Error`].
	#[serde(rename = "r")]
	HeadsRequest(CoId),

	/// Error notification.
	#[serde(rename = "e")]
	Error { code: HeadsErrorCode, message: String },
}
impl HeadsMessage {
	/// DIDComm message header.
	pub fn create_header() -> DidCommHeader {
		let mut header = DidCommHeader::new(format!("co-heads/1.0.0"));
		header.expires_time = header.created_time.map(|t| t + 120);
		header
	}
}

#[derive(Debug, Clone, Serialize_repr, Deserialize_repr)]
#[non_exhaustive]
#[repr(u16)]
pub enum HeadsErrorCode {
	Forbidden = 403,
	ServiceUnavailable = 503,
}
