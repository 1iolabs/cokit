// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::library::application_actor::{ApplicationActorMessage, CreateIdentityRequest};
use co_actor::ActorHandle;
use tauri::ipc::InvokeError;

#[tauri::command]
pub(crate) async fn create_identity(
	actor_handle: tauri::State<'_, ActorHandle<ApplicationActorMessage>>,
	name: String,
	seed: Option<Vec<u8>>,
) -> Result<String, InvokeError> {
	let result = actor_handle
		.request(|r| ApplicationActorMessage::CreateIdentity(CreateIdentityRequest { name, seed }, r))
		.await
		.map_err(InvokeError::from_error)?
		.map_err(InvokeError::from_anyhow)?;
	Ok(result)
}
