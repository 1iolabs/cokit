use libipld::{Cid, Ipld};
use std::collections::BTreeMap;

#[derive(Default, Debug, Clone)]
pub struct CoState {
    pub root: Option<Cid>,
    pub settings: BTreeMap<String, Ipld>,
}
