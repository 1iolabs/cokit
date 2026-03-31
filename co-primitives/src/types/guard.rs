// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::Tags;
use cid::Cid;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Serialize, Deserialize)]
pub struct GuardInput {
	/// Gurad name which references this guard in [`co_core_co::Co::guards`].
	pub guard: String,

	/// The state to check the guard against
	pub state: Cid,

	/// The heads that produced the state
	pub heads: BTreeSet<Cid>,

	/// The head to check
	pub next_head: Cid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GuardOutput {
	/// Gurad result.
	pub result: bool,

	/// Error if the guard has failed.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub error: Option<String>,

	/// Guard Metadata
	#[serde(default, skip_serializing_if = "Tags::is_empty")]
	pub tags: Tags,
}
