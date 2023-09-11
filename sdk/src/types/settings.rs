use crate::CoState;
use libipld::{Cid, Ipld};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize)]
pub struct JsonSettings {
	pub root: Option<Cid>,
	pub settings: Option<BTreeMap<String, Ipld>>,
}

impl From<CoState> for JsonSettings {
	fn from(value: CoState) -> Self {
		Self {
			root: value.root,
			settings: match value.settings.len() {
				0 => None,
				_ => Some(value.settings),
			},
		}
	}
}
