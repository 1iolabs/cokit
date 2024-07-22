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
	pub peer: Vec<u8>,

	/// CO Connectivity
	pub network: CoConnectivity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoConnectivity {
	/// Networks to connect to.
	/// Maybe empty.
	#[serde(rename = "n")]
	pub network: BTreeSet<Network>,

	/// Participants to connect to.
	/// Maybe empty.
	/// Network should be preferred.
	#[serde(rename = "p")]
	pub participants: BTreeSet<Did>,
}
