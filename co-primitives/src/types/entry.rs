// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::Clock;
use cid::Cid;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Entry {
	/// The stream id.
	/// Todo: Do we need this?
	#[serde(rename = "i", with = "serde_bytes")]
	pub id: Vec<u8>,
	#[serde(rename = "p")]
	pub payload: Cid,
	#[serde(rename = "n")]
	pub next: BTreeSet<Cid>,
	#[serde(rename = "r", default, skip_serializing_if = "BTreeSet::is_empty")]
	pub refs: BTreeSet<Cid>,
	#[serde(rename = "c")]
	pub clock: Clock,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SignedEntry {
	/// The identity.
	#[serde(rename = "u")]
	pub identity: String,

	/// Identity public key.
	#[serde(rename = "k", default, with = "serde_bytes", skip_serializing_if = "Option::is_none")]
	pub public_key: Option<Vec<u8>>,

	/// The identity.
	#[serde(rename = "s", with = "serde_bytes")]
	pub signature: Vec<u8>,

	/// Entry.
	#[serde(rename = "e")]
	// note: this causes serde to write unbounded maps which are indefinite length maps which are not supported in
	// DAG-CBOR. #[serde(flatten)]
	pub entry: Entry,
}
