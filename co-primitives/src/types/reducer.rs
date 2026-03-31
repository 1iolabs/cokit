// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::Tags;
use cid::Cid;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ReducerInput {
	/// Source state.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub state: Option<Cid>,

	/// [`crate::ReducerAction`] to reduce.
	pub action: Cid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReducerOutput {
	/// Result state.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub state: Option<Cid>,

	/// Error if the reducer has failed.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub error: Option<String>,

	/// Reducer metadata.
	/// Tags retuned here will be merged into core tags.
	#[serde(default, skip_serializing_if = "Tags::is_empty")]
	pub tags: Tags,
}
