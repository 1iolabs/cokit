use co_identity::{DidCommHeader, Identity, PrivateIdentity};
use co_network::didcomm::EncodedMessage;
use co_primitives::{to_json_string, CoId};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};

pub const CO_DIDCOMM_KEY_REQUEST: &str = "co-key-request";
pub const CO_DIDCOMM_KEY_RESPONSE: &str = "co-key-response";

/// Create an signed key request message.
pub fn create_key_request_message<F>(from: &F, payload: KeyRequestPayload) -> anyhow::Result<(String, EncodedMessage)>
where
	F: PrivateIdentity + Send + Sync + 'static,
{
	let (from_didcomm, header) = DidCommHeader::create_from(from, CO_DIDCOMM_KEY_REQUEST)?;
	let id = header.id.clone();
	let body = to_json_string(&payload)?;
	let message = from_didcomm.jws(header, &body)?;
	Ok((id, EncodedMessage(message.into_bytes())))
}

/// Create an encrypted key response message.
pub fn create_key_response_message<F, T>(
	from: &F,
	to: &T,
	request_message_id: String,
	payload: KeyResponsePayload,
) -> anyhow::Result<EncodedMessage>
where
	F: PrivateIdentity + Send + Sync + 'static,
	T: Identity + Send + Sync + 'static,
{
	let (from_didcomm, to_didcomm, mut header) = DidCommHeader::create(from, to, CO_DIDCOMM_KEY_RESPONSE)?;
	header.thid = Some(request_message_id);
	let body = to_json_string(&payload)?;
	let message = from_didcomm.jwe(&to_didcomm, header, &body)?;
	Ok(EncodedMessage(message.into_bytes()))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyRequestPayload {
	/// The requesters PeerId.
	/// When signed this creates an relation between the DID and the PeerID to enable receiver trust.
	pub peer: PeerId,
	pub id: CoId,
	pub key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum KeyResponsePayload {
	Ok(co_core_keystore::Key),
	Failure,
}
