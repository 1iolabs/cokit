use crate::{CoAction, CoState};
use libipld::{Cid, Ipld};

pub fn reducer(state: CoState, action: &CoAction) -> CoState {
    use CoAction::*;
    match action {
        RootChanged(id, _) => root_changed(state, id),
        SettingChanged(key, value, _) => setting_changed(state, key, value),
        _ => state,
    }
}

fn root_changed(state: CoState, id: &Cid) -> CoState {
    CoState {
        root: Some(id.clone()),
        ..state
    }
}

fn setting_changed(state: CoState, key: &String, value: &Ipld) -> CoState {
    let mut settings = state.settings;
    settings.insert(key.clone(), value.clone());
    CoState { settings, ..state }
}
