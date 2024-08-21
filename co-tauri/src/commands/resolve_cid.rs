use crate::library::tauri_error::CoTauriError;
use co_sdk::{Application, BlockStorageExt, CoId, CoReducerFactory};
use libipld::{Cid, Ipld};

#[tauri::command]
pub(crate) async fn resolve_cid(
	application: tauri::State<'_, Application>,
	cid: Cid,
	co: CoId,
) -> Result<Ipld, CoTauriError> {
	let storage = application
		.context()
		.try_co_reducer(&co)
		.await
		.map_err(|err| anyhow::Error::from(err))?
		.storage();
	let ipld: Ipld = storage.get_deserialized::<Ipld>(&cid).await?;
	Ok(ipld)
}
