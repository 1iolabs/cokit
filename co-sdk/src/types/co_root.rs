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
