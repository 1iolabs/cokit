// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use co_identity::{DidCommHeader, PrivateIdentity};
use co_network::EncodedMessage;
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
pub fn create_join_message_from<F>(
	from: &F,
	co: CoId,
	thid: Option<String>,
) -> anyhow::Result<(DidCommHeader, EncodedMessage)>
where
	F: PrivateIdentity + Send + Sync + 'static,
{
	let (from_didcomm, mut header) = DidCommHeader::create_from(from, CO_DIDCOMM_JOIN)?;
	header.thid = thid;
	let payload = CoJoinPayload { id: co };
	let body = to_json_string(&payload)?;
	let message = from_didcomm.jws(header.clone(), &body)?;
	Ok((header, EncodedMessage(message.into_bytes())))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CoJoinPayload {
	pub id: CoId,
}
