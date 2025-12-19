use crate::library::application_actor::{ApplicationActorMessage, SessionId};
use co_actor::ActorHandle;
use co_sdk::CoId;
use tauri::ipc::InvokeError;

#[tauri::command]
pub(crate) async fn session_open(
	actor_handle: tauri::State<'_, ActorHandle<ApplicationActorMessage>>,
	co_id: CoId,
) -> Result<SessionId, InvokeError> {
	actor_handle
		.request(|r| ApplicationActorMessage::SessionOpen(co_id, r))
		.await
		.map_err(InvokeError::from_error)?
		.map_err(InvokeError::from_anyhow)
}

#[tauri::command]
pub(crate) async fn session_close(
	actor_handle: tauri::State<'_, ActorHandle<ApplicationActorMessage>>,
	session_id: SessionId,
) -> Result<(), InvokeError> {
	actor_handle
		.dispatch(ApplicationActorMessage::SessionClose(session_id))
		.map_err(InvokeError::from_error)
}
