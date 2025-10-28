use cid::Cid;
use co_identity::{DidCommHeader, Identity, PrivateIdentity};
use co_network::didcomm::EncodedMessage;
use co_primitives::{to_json_string, CoConnectivity, CoId, Tags};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const CO_DIDCOMM_INVITE: &str = "co-invite";

/// Create an encoded invite message.
pub fn create_invite_message<F, T>(
	from: &F,
	to: &T,
	co: CoInvitePayload,
	thid: Option<String>,
) -> anyhow::Result<(DidCommHeader, EncodedMessage)>
where
	F: PrivateIdentity + Send + Sync + 'static,
	T: Identity + Send + Sync + 'static,
{
	let (from_didcomm, to_didcomm, mut header) = DidCommHeader::create(from, to, CO_DIDCOMM_INVITE)?;
	header.thid = thid;
	let body = to_json_string(&co)?;
	let message = from_didcomm.jwe(&to_didcomm, header.clone(), &body)?;
	Ok((header, EncodedMessage(message.into_bytes())))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CoInvitePayload {
	/// The CO ID.
	pub id: CoId,

	/// The invite tags.
	pub tags: Tags,

	/// The latest known CO State (encrypted if the CO is not public).
	pub state: Cid,

	/// The latest known CO Heads (encrypted if the CO is not public).
	pub heads: BTreeSet<Cid>,

	/// Connectivity settings.
	pub connectivity: CoConnectivity,
}
