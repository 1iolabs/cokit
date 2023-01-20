use libipld::Cid;
use serde::{Serialize, Deserialize};

/// Application state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub root: Option<Cid>,
}
