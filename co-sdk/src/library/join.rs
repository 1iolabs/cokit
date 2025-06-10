use co_identity::{DidCommHeader, PrivateIdentity};
use co_network::didcomm::EncodedMessage;
use co_primitives::{to_json_string, CoId};
use serde::{Deserialize, Serialize};

pub const CO_DIDCOMM_JOIN: &str = "co-join";

// /// Create an encoded (encrypted) join message.
// pub fn create_join_message<F, T>(from: &F, to: &T, co: CoId, thid: Option<String>) -> anyhow::Result<EncodedMessage>
// where
// 	F: PrivateIdentity + Send + Sync + 'static,
// 	T: Identity + Send + Sync + 'static,
// {
// 	let (from_didcomm, to_didcomm, mut header) = DidCommHeader::create(from, to, CO_DIDCOMM_JOIN)?;
// 	header.thid = thid;
// 	let body = to_json_string(&co)?;
// 	let message = from_didcomm.jwe(&to_didcomm, header, &body)?;
// 	Ok(EncodedMessage(message.into_bytes()))
// }

/// Create an encoded (signed) join message to unknown receipents.
pub fn create_join_message_from<F>(from: &F, co: CoId, thid: Option<String>) -> anyhow::Result<(String, EncodedMessage)>
where
	F: PrivateIdentity + Send + Sync + 'static,
{
	let (from_didcomm, mut header) = DidCommHeader::create_from(from, CO_DIDCOMM_JOIN)?;
	header.thid = thid;
	let payload = CoJoinPayload { id: co };
	let body = to_json_string(&payload)?;
	let message_id = header.id.clone();
	let message = from_didcomm.jws(header, &body)?;
	Ok((message_id, EncodedMessage(message.into_bytes())))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CoJoinPayload {
	pub id: CoId,
}
