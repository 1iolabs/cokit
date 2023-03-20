use crate::{types::settings::JsonSettings, Cause, CoAction, CoContext, CoState, ErrorKind};
use co_state::{ActionObservable, StateObservable};
use rxrust::prelude::*;
use std::{convert::Infallible, path::Path, sync::Arc, time::Duration};

pub fn store_settings<O: Observer<CoAction, Infallible> + 'static>(
    actions: ActionObservable<CoAction>,
    states: StateObservable<CoState>,
    context: Arc<CoContext>,
) -> impl Observable<CoAction, Infallible, O> {
    actions
        .clone()
        .filter_map(|action| match action {
            CoAction::Initialize(path) => Some(path),
            _ => None,
        })
        .take(1)
        .flat_map(move |path| {
            let inner_context = context.clone();
            actions
                .clone()
                .filter_map(|action2| -> Option<CoAction> {
                    match action2 {
                        CoAction::RootChanged(_, Cause::Change) => Some(action2),
                        CoAction::SettingChanged(_, _, Cause::Change) => Some(action2),
                        _ => None,
                    }
                })
                .buffer_with_time(Duration::from_millis(100), context.scheduler())
                .filter(|_a: &Vec<CoAction>| true)
                .with_latest_from(states.clone())
                .flat_map(move |(_actions, state)| {
                    from_future(
                        store_settings_to_path(path.clone(), state.into()),
                        inner_context.scheduler(),
                    )
                    .filter_map(move |result| -> Option<CoAction> {
                        match result {
                            Ok(_) => None,
                            Err(e) => {
                                Some(CoAction::Error(e.to_string(), ErrorKind::Warning.into()))
                            }
                        }
                    })
                })
        })
}

#[tracing::instrument(
    // name = "store_settings_to_path",
    skip(
        settings,
    ),
    fields(
        path = path.as_ref().to_str(),
    ),
)]
async fn store_settings_to_path(
    path: impl AsRef<Path>,
    settings: JsonSettings,
) -> anyhow::Result<()> {
    // serialize
    let contents = serde_json::to_string(&settings)?;

    // store
    tokio::fs::write(path, contents).await?;

    // result
    Ok(())
}
