use libipld::{Cid, Ipld};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Default, Debug, Clone)]
pub struct CoState {
    pub base_path: PathBuf,
    pub root: Option<Cid>,
    pub settings: BTreeMap<String, Ipld>,
}

impl CoState {
    pub fn new(base_path: PathBuf) -> Self {
        CoState {
            base_path,
            ..Default::default()
        }
    }
}
