// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::CoReducerState;
use cid::Cid;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Serializable CO root reducer state
/// See:
/// - [`CoPinningKey::Root`]
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CoRoot {
	#[serde(rename = "h", default, skip_serializing_if = "BTreeSet::is_empty")]
	pub heads: BTreeSet<Cid>,
	#[serde(rename = "s", default, skip_serializing_if = "Option::is_none")]
	pub state: Option<Cid>,
}
impl From<CoReducerState> for CoRoot {
	fn from(value: CoReducerState) -> Self {
		CoRoot { heads: value.1, state: value.0 }
	}
}
impl From<CoRoot> for CoReducerState {
	fn from(value: CoRoot) -> Self {
		CoReducerState::new(value.state, value.heads)
	}
}
