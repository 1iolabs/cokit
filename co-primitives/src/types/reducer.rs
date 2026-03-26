// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

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
