use crate::library::tauri_error::CoTauriError;
use co_sdk::{Application, CoId, CoReducerFactory};
use libipld::Cid;
use std::collections::BTreeSet;

#[tauri::command]
pub(crate) async fn get_co_state(
	application: tauri::State<'_, Application>,
	co: CoId,
) -> Result<(Option<Cid>, BTreeSet<Cid>), CoTauriError> {
	let reducer = application
		.context()
		.try_co_reducer(&co)
		.await
		.map_err(|err| anyhow::Error::from(err))?;
	let (state, heads) = reducer.reducer_state().await;
	Ok((state, heads))
}
