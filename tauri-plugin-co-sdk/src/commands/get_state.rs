// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::library::application_actor::ApplicationActorMessage;
use co_actor::ActorHandle;
use co_sdk::{to_cbor, CoId};
use tauri::ipc::{InvokeError, Response};

#[tauri::command]
pub(crate) async fn get_co_state(
	actor_handle: tauri::State<'_, ActorHandle<ApplicationActorMessage>>,
	co: CoId,
) -> Result<Response, InvokeError> {
	let result = actor_handle
		.request(|r| ApplicationActorMessage::GetCoState(co, r))
		.await
		.map_err(InvokeError::from_error)?
		.map_err(InvokeError::from_anyhow)?;

	Ok(Response::new(to_cbor(&result).map_err(InvokeError::from_error)?))
}
