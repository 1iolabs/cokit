use crate::library::{
	application_actor::{ApplicationActorMessage, SessionId},
	tauri_error::CoTauriError,
};
use cid::Cid;
use co_actor::ActorHandle;
use co_primitives::{Block, DefaultParams};

#[tauri::command]
pub(crate) async fn storage_get(
	actor_handle: tauri::State<'_, ActorHandle<ApplicationActorMessage>>,
	session_id: SessionId,
	cid: Cid,
) -> Result<Vec<u8>, CoTauriError> {
	Ok(actor_handle
		.request(|r| ApplicationActorMessage::StorageGet(session_id, cid, r))
		.await??
		.data()
		.into())
}

#[tauri::command]
pub(crate) async fn storage_set(
	actor_handle: tauri::State<'_, ActorHandle<ApplicationActorMessage>>,
	session_id: SessionId,
	cid: Cid,
	data: Vec<u8>,
) -> Result<Cid, CoTauriError> {
	let block = Block::<DefaultParams>::new(cid, data)?;
	Ok(actor_handle
		.request(|r| ApplicationActorMessage::StorageSet(session_id, block, r))
		.await??)
}
