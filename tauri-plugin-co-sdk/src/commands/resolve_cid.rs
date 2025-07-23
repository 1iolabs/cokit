use crate::library::application_actor::{ApplicationActorMessage, SessionId};
use cid::Cid;
use co_actor::ActorHandle;
use co_sdk::{from_cbor, to_cbor};
use serde::Deserialize;
use tauri::ipc::{InvokeError, Response};

#[derive(Deserialize, Debug)]
struct ResolveCidBody {
	session: SessionId,
	cid: Cid,
}

#[tauri::command]
pub(crate) async fn resolve_cid(
	actor_handle: tauri::State<'_, ActorHandle<ApplicationActorMessage>>,
	body: Vec<u8>,
) -> Result<Response, InvokeError> {
	let body: ResolveCidBody = from_cbor(&body).map_err(InvokeError::from_error)?;
	// tracing::debug!("tauri resolve cid {:#?}", body_ipld);
	// let body = from_ipld::<ResolveCidBody>(body_ipld).map_err(InvokeError::from_error)?;
	let result = actor_handle
		.request(|r| ApplicationActorMessage::ResolveCid(body.session, body.cid, r))
		.await
		.map_err(InvokeError::from_error)?
		.map_err(InvokeError::from_anyhow)?;
	Ok(Response::new(to_cbor(&result).map_err(InvokeError::from_error)?))
}
