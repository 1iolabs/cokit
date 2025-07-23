use crate::library::application_actor::{ApplicationActorMessage, GetActionsRequest};
use co_actor::ActorHandle;
use co_sdk::{from_cbor, to_cbor};
use tauri::ipc::InvokeError;

#[tauri::command]
pub(crate) async fn get_actions(
	actor_handle: tauri::State<'_, ActorHandle<ApplicationActorMessage>>,
	body: Vec<u8>,
) -> Result<tauri::ipc::Response, InvokeError> {
	let body: GetActionsRequest = from_cbor(&body).map_err(InvokeError::from_error)?;
	let result = actor_handle
		.request(|r| ApplicationActorMessage::GetActions(body, r))
		.await
		.map_err(InvokeError::from_error)?
		.map_err(InvokeError::from_anyhow)?;
	Ok(tauri::ipc::Response::new(to_cbor(&result).map_err(InvokeError::from_error)?))
}
