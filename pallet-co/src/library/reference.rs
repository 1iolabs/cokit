use libipld::Cid;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ListReference {
    /// The referenced items.
    #[serde(rename = "v")]
    pub version: ListReferenceVersion,

    /// The referenced items.
    #[serde(rename = "r")]
    pub reference: Cid,

    /// Linked list to next `ListReference`.
    #[serde(rename = "n")]
    pub next: Option<Cid>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub enum ListReferenceVersion {
    #[default]
    V1 = 1,
}
