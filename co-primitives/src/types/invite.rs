// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{Did, IsDefault, Network};
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
