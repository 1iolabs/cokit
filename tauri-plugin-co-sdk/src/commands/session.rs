use crate::library::{
	application_actor::{ApplicationActorMessage, SessionId},
	tauri_error::CoTauriError,
};
use co_actor::ActorHandle;
use co_sdk::CoId;

#[tauri::command]
pub(crate) async fn session_open(
	actor_handle: tauri::State<'_, ActorHandle<ApplicationActorMessage>>,
	co_id: CoId,
) -> Result<SessionId, CoTauriError> {
	Ok(actor_handle
		.request(|r| ApplicationActorMessage::SessionOpen(co_id, r))
		.await??)
}

#[tauri::command]
pub(crate) async fn session_close(
	actor_handle: tauri::State<'_, ActorHandle<ApplicationActorMessage>>,
	session_id: SessionId,
) -> Result<(), CoTauriError> {
	Ok(actor_handle.dispatch(ApplicationActorMessage::SessionClose(session_id))?)
}
