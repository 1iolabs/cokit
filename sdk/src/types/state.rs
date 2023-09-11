use crate::CoExecuteState;
use libipld::{Cid, Ipld};
use serde::Serialize;
use std::{
	collections::{BTreeMap, HashMap},
	path::PathBuf,
};

#[derive(Default, Debug, Clone, Serialize)]
pub struct CoState {
	pub base_path: PathBuf,
	pub root: Option<Cid>,
	pub settings: CoSettings,
	pub execute: HashMap<String, CoExecuteState>,
}

pub type CoSettings = BTreeMap<String, Ipld>;

impl CoState {
	pub fn new(base_path: PathBuf) -> Self {
		CoState { base_path, ..Default::default() }
	}
}
