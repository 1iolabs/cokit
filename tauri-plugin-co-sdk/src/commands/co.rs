// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::library::application_actor::{ApplicationActorMessage, CreateCoRequest};
use co_actor::ActorHandle;
use co_sdk::CoId;
use tauri::ipc::InvokeError;
use uuid::Uuid;

#[tauri::command]
pub(crate) async fn create_co(
	actor_handle: tauri::State<'_, ActorHandle<ApplicationActorMessage>>,
	creator_did: String,
	co_name: String,
	public: bool,
	co_id: Option<CoId>,
) -> Result<CoId, InvokeError> {
	let used_co_id = co_id.unwrap_or(Uuid::new_v4().to_string().into());
	actor_handle
		.dispatch(ApplicationActorMessage::CreateCo(CreateCoRequest {
			creator_did,
			co_id: used_co_id.clone(),
			co_name,
			public,
		}))
		.map_err(InvokeError::from_error)?;
	Ok(used_co_id)
}
