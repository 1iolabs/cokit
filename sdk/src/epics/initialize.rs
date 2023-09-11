use crate::types::{
	action::{Cause, CoAction},
	context::CoContext,
	error::{ErrorKind, IntoAction},
	settings::JsonSettings,
	state::CoState,
};
use anyhow::{Error, Result};
use co_state::{ActionObservable, EndWithExt, StateObservable};
use rxrust::prelude::*;
use std::{convert::Infallible, path::Path, sync::Arc};
use tokio::fs::read_to_string;

/// Load configuration on startup.
///
/// In: CoAction::Initialize
/// Out: CoAction::Error, CoAction::RootChanged, CoAction::SettingChanged, CoAction::Initialized
pub fn initialize<O: Observer<CoAction, Infallible> + 'static>(
	actions: ActionObservable<CoAction>,
	states: StateObservable<CoState>,
	context: Arc<CoContext>,
) -> impl Observable<CoAction, Infallible, O> {
	actions
		.filter(|action| *action == CoAction::Initialize)
		.with_latest_from(states)
		.take(1)
		.flat_map(move |(_, state)| {
			observable::from_future(load_settings_from_path(state.base_path.join("state.json")), context.scheduler())
		})
		.flat_map(|result| from_iter(result.into_action(ErrorKind::Fatal)))
		.end_with(vec![CoAction::Initialized])
}

#[tracing::instrument(
    // name = "load_settings_from_path",
    fields(
        path = path.as_ref().to_str(),
    ),
)]
async fn load_settings_from_path(path: impl AsRef<Path>) -> Result<Vec<CoAction>> {
	// log
	tracing::event!(tracing::Level::INFO, path = path.as_ref().to_str(), "load_settings_from_path");

	// log
	let mut result = Vec::new();
	let data = match read_to_string(path.as_ref()).await {
		Ok(data) => data,
		Err(e) => {
			return match e.kind() {
				std::io::ErrorKind::NotFound => Ok(result),
				// _ => {
				//     result.push(CoAction::Error(format!("Open file: {}",
				// path.as_ref().to_str().unwrap_or("unknown")), ErrorKind::Fatal.into()));     result
				// }
				_ => Err(Error::from(e).context(format!("Open file: {}", path.as_ref().to_str().unwrap_or("unknown")))),
			}
		},
	};
	let data: JsonSettings = serde_json::from_str(&data)?;
	if let Some(cid) = data.root {
		result.push(CoAction::RootChanged(cid, Cause::Initialize));
	}
	if let Some(settings) = data.settings {
		for (key, value) in settings {
			result.push(CoAction::SettingChanged(key, value, Cause::Initialize));
		}
	}
	Ok(result)
}
