// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use co_identity::{DidCommHeader, Identity, PeerDidCommHeader, PrivateIdentity};
use co_network::{EncodedMessage, PeerId};
use co_primitives::CoDateRef;
use std::collections::BTreeMap;

pub const CO_DIDCOMM_CONTACT: &str = "co-contact";

/// Create an encoded contact message.
pub fn create_contact_message<F, T>(
	date: &CoDateRef,
	from_peer: PeerId,
	from: &F,
	to: &T,
	sub: Option<String>,
	fields: BTreeMap<String, String>,
) -> anyhow::Result<(DidCommHeader, EncodedMessage)>
where
	F: PrivateIdentity + Send + Sync + 'static,
	T: Identity + Send + Sync + 'static,
{
	let (from_didcomm, to_didcomm, header) = DidCommHeader::create(date, from, to, CO_DIDCOMM_CONTACT)?;
	let mut header = header.with_fields(fields)?;
	if let Some(sub) = sub {
		header.fields.insert("sub".to_owned(), sub);
	}
	let message_header: DidCommHeader = PeerDidCommHeader { header, from_peer_id: Some(from_peer.to_string()) }.into();
	let message = from_didcomm.jwe(&to_didcomm, message_header.clone(), "null")?;
	Ok((message_header, EncodedMessage(message.into_bytes())))
}
