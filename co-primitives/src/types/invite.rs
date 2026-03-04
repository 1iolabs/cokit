// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{Did, Network};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoInviteMetadata {
	/// Invite message ID.
	pub id: String,

	/// Invite remote sender.
	pub from: Did,

	/// Invite remote peer.
	#[serde(with = "serde_bytes")]
	pub peer: Option<Vec<u8>>,

	/// CO Connectivity
	#[serde(default, skip_serializing_if = "IsDefault::is_default")]
	pub network: CoConnectivity,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct CoConnectivity {
	/// Networks to connect to.
	/// Maybe empty.
	#[serde(rename = "n", default, skip_serializing_if = "BTreeSet::is_empty")]
	pub network: BTreeSet<Network>,

	/// Participants to connect to.
	/// Maybe empty.
	/// Network should be preferred.
	#[serde(rename = "p", default, skip_serializing_if = "BTreeSet::is_empty")]
	pub participants: BTreeSet<Did>,
}
