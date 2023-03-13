use crate::types::{
    action::CoAction,
    context::CoContext,
    error::{ErrorKind, IntoAction},
    state::CoState,
};
use anyhow::{Error, Result};
use co_state::{ActionObservable, StateObservable};
use libipld::{Cid, Ipld};
use rxrust::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::{convert::Infallible, path::Path, sync::Arc};
use tokio::fs::read_to_string;

pub fn initialize<O: Observer<CoAction, Infallible> + 'static>(
    actions: ActionObservable<CoAction>,
    _states: StateObservable<CoState>,
    context: Arc<CoContext>,
) -> impl Observable<CoAction, Infallible, O> {
    actions
        .filter_map(|action| match action {
            CoAction::Initialize(path) => Some(path),
            _ => None,
        })
        .take(1)
        .flat_map(move |path| context.from_future(load_settings_from_path(path)))
        .flat_map(move |result| from_iter(result.into_action(ErrorKind::Fatal)))
}

async fn load_settings_from_path(path: impl AsRef<Path>) -> Result<Vec<CoAction>> {
    let mut result = Vec::new();
    let data = match read_to_string(path.as_ref()).await {
        Ok(data) => data,
        Err(e) => {
            return match e.kind() {
                std::io::ErrorKind::NotFound => Ok(result),
                // _ => {
                //     result.push(CoAction::Error(format!("Open file: {}", path.as_ref().to_str().unwrap_or("unknown")), ErrorKind::Fatal.into()));
                //     result
                // }
                _ => Err(Error::from(e).context(format!(
                    "Open file: {}",
                    path.as_ref().to_str().unwrap_or("unknown")
                ))),
            };
        }
    };
    let data: JsonSettings = serde_json::from_str(&data)?;
    if let Some(cid) = data.root {
        result.push(CoAction::RootChanged(cid));
    }
    if let Some(settings) = data.settings {
        for (key, value) in settings {
            result.push(CoAction::SettingChanged(key, value));
        }
    }
    Ok(result)
}

#[derive(Serialize, Deserialize)]
struct JsonSettings {
    pub root: Option<Cid>,
    pub settings: Option<BTreeMap<String, Ipld>>,
}
