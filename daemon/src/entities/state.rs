use libipld::Cid;

/// Application state.
#[derive(Debug, Clone)]
pub struct State {
    pub root: Option<Cid>,
}
