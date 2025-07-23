use crate::library::application_actor::{ApplicationActorMessage, CreateIdentityRequest};
use co_actor::ActorHandle;
use tauri::ipc::InvokeError;

#[tauri::command]
pub(crate) async fn create_identity(
	actor_handle: tauri::State<'_, ActorHandle<ApplicationActorMessage>>,
	name: String,
	seed: Option<Vec<u8>>,
) -> Result<(), InvokeError> {
	actor_handle
		.dispatch(ApplicationActorMessage::CreateIdentity(CreateIdentityRequest { name, seed }))
		.map_err(InvokeError::from_error)?;
	Ok(())
}
