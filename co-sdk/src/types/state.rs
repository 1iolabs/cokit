use crate::CoExecuteState;
use libipld::{Cid, Ipld};
use serde::Serialize;
use std::{
	collections::{BTreeMap, HashMap},
	path::PathBuf,
};

#[derive(Default, Debug, Clone, Serialize)]
pub struct CoState {
	/// Config storage folder path.
	pub config_path: PathBuf,
	/// Data storage folder path.
	pub data_path: PathBuf,
	/// References the local CO list.
	pub root: Option<Cid>,
	/// Locally persisted application settings.
	pub settings: CoSettings,
	/// CO runtime states.
	pub execute: HashMap<String, CoExecuteState>,
	/// Currently registered didcontact rendezvous points.
	pub didcontact: Vec<String>,
}

pub type CoSettings = BTreeMap<String, Ipld>;

impl CoState {
	pub fn new(config_path: PathBuf, data_path: PathBuf) -> Self {
		CoState { config_path, data_path, ..Default::default() }
	}
}
