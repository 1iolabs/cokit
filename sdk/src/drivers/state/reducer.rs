use crate::{CoAction, CoExecuteState, CoState};
use libipld::{Cid, Ipld};

pub fn reducer(state: CoState, action: &CoAction) -> CoState {
	use CoAction::*;
	match action {
		RootChanged(id, _) => root_changed(state, id),
		SettingChanged(key, value, _) => setting_changed(state, key, value),
		CoStartup { id: _ } | CoShutdown { id: _ } | CoExecuteStateChanged { id: _, state: _ } =>
			execute(state, action),
		_ => state,
	}
}

fn execute(state: CoState, action: &CoAction) -> CoState {
	let change = match action {
		CoAction::CoStartup { id } => Some((id, CoExecuteState::Starting)),
		CoAction::CoShutdown { id } => Some((id, CoExecuteState::Stopping)),
		CoAction::CoExecuteStateChanged { id, state } => Some((id, state.clone())),
		_ => None,
	};
	if let Some((id, execute_state)) = change {
		if state.execute.get(id) != Some(&execute_state) {
			let mut execute = state.execute.clone();
			if execute_state == CoExecuteState::default() {
				execute.remove(id);
			} else {
				execute.insert(id.clone(), execute_state);
			}
			return CoState { execute, ..state }
		}
	}
	state
}

fn root_changed(state: CoState, id: &Cid) -> CoState {
	CoState { root: Some(id.clone()), ..state }
}

fn setting_changed(state: CoState, key: &String, value: &Ipld) -> CoState {
	let mut settings = state.settings;
	settings.insert(key.clone(), value.clone());
	CoState { settings, ..state }
}
