use std::collections::BTreeSet;

use anyhow::anyhow;
use co_sdk::CoId;

use co_sdk::Application;
use libipld::Cid;

use crate::library::tauri_error::CoTauriError;

#[tauri::command]
pub(crate) async fn get_co_state(
	application: tauri::State<'_, Application>,
	co: CoId,
) -> Result<(Option<Cid>, BTreeSet<Cid>), CoTauriError> {
	let reducer = application
		.co_reducer(co.clone())
		.await?
		.ok_or(anyhow!("Co not found: {}", co.clone()))?;
	let (state, heads) = reducer.reducer_state().await;
	Ok((state, heads))
}
