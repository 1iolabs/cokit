use libipld::Cid;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HeadsMessage {
	#[serde(rename = "h")]
	Heads(BTreeSet<Cid>),
}
