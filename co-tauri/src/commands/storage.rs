use crate::library::{application_actor::ApplicationActorMessage, tauri_error::CoTauriError};
use co_actor::ActorHandle;
use co_sdk::CoId;
use libipld::{Block, Cid, DefaultParams};

#[tauri::command]
async fn storage_get(
	actor_handle: tauri::State<'_, ActorHandle<ApplicationActorMessage>>,
	co_id: CoId,
	cid: Cid,
) -> Result<Block<DefaultParams>, CoTauriError> {
	Ok(actor_handle
		.request(|r| ApplicationActorMessage::StorageGet(co_id, cid, r))
		.await??)
}

#[tauri::command]
async fn storage_set(
	actor_handle: tauri::State<'_, ActorHandle<ApplicationActorMessage>>,
	co_id: CoId,
	block: Block<DefaultParams>,
) -> Result<Cid, CoTauriError> {
	Ok(actor_handle
		.request(|r| ApplicationActorMessage::StorageSet(co_id, block, r))
		.await??)
}
