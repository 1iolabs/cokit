use cid::Cid;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Serialize, Deserialize)]
pub struct GuardVerifyPayload {
	pub guard: String,
	pub state: Cid,
	pub heads: BTreeSet<Cid>,
	pub next_head: Cid,
}
