use anyhow::anyhow;
use co_sdk::{Application, BlockStorageExt, CoId};
use libipld::{Cid, Ipld};

use crate::library::tauri_error::CoTauriError;

#[tauri::command]
pub(crate) async fn resolve_cid(
	application: tauri::State<'_, Application>,
	cid: Cid,
	co: CoId,
) -> Result<Ipld, CoTauriError> {
	let storage = application
		.co_reducer(co.clone())
		.await?
		.ok_or(anyhow!("Co not found: {:#?}", co.clone()))?
		.storage();
	let ipld: Ipld = storage.get_deserialized::<Ipld>(&cid).await?;
	Ok(ipld)
}
