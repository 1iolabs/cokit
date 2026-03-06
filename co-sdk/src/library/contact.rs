// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use co_identity::{DidCommHeader, Identity, PrivateIdentity};
use co_network::EncodedMessage;
use co_primitives::{to_json_string, CoDateRef};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const CO_DIDCOMM_CONTACT: &str = "co-contact";

/// Create an encoded contact message.
pub fn create_contact_message<F, T>(
	date: &CoDateRef,
	from: &F,
	to: &T,
	payload: ContactPayload,
) -> anyhow::Result<(DidCommHeader, EncodedMessage)>
where
	F: PrivateIdentity + Send + Sync + 'static,
	T: Identity + Send + Sync + 'static,
{
	let (from_didcomm, to_didcomm, header) = DidCommHeader::create(date, from, to, CO_DIDCOMM_CONTACT)?;
	let body = to_json_string(&payload)?;
	let message = from_didcomm.jwe(&to_didcomm, header.clone(), &body)?;
	Ok((header, EncodedMessage(message.into_bytes())))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContactPayload {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub sub: Option<String>,

	#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
	pub fields: BTreeMap<String, String>,
}
