use crate::library::{application_actor::ApplicationActorMessage, tauri_error::CoTauriError};
use co_actor::ActorHandle;

#[tauri::command]
pub async fn create_identity(
	actor_handle: tauri::State<'_, ActorHandle<ApplicationActorMessage>>,
	name: String,
) -> Result<(), CoTauriError> {
	actor_handle.dispatch(ApplicationActorMessage::CreateIdentity(name, None))?;
	Ok(())
}
