use crate::library::{
	application_actor::{ApplicationActorMessage, CreateIdentityRequest},
	tauri_error::CoTauriError,
};
use co_actor::ActorHandle;

#[tauri::command]
pub(crate) async fn create_identity(
	actor_handle: tauri::State<'_, ActorHandle<ApplicationActorMessage>>,
	name: String,
	seed: Option<Vec<u8>>,
) -> Result<(), CoTauriError> {
	actor_handle.dispatch(ApplicationActorMessage::CreateIdentity(CreateIdentityRequest { name, seed }))?;
	Ok(())
}
