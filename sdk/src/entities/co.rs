use std::{collections::BTreeMap};
use serde::{Serialize, Deserialize};
use libipld::ipld::Ipld;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Co {
    pub id: String,
    pub name: String,
    pub data: BTreeMap<String, Ipld>,
}
