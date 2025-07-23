use crate::library::application_actor::{ApplicationActorMessage, SessionId};
use cid::Cid;
use co_actor::ActorHandle;
use co_primitives::{Block, DefaultParams};
use co_sdk::{from_cbor, to_cbor, KnownMultiCodec};
use serde::Deserialize;
use tauri::ipc::{InvokeError, Response};

#[derive(Debug, Deserialize)]
pub struct StorageGetBody {
	session: SessionId,
	cid: Cid,
}

#[tauri::command]
pub(crate) async fn storage_get(
	actor_handle: tauri::State<'_, ActorHandle<ApplicationActorMessage>>,
	body: Vec<u8>,
) -> Result<Response, InvokeError> {
	let body: StorageGetBody = from_cbor(&body).map_err(InvokeError::from_error)?;
	let data: Vec<u8> = actor_handle
		.request(|r| ApplicationActorMessage::StorageGet(body.session, body.cid, r))
		.await
		.map_err(InvokeError::from_error)?
		.map_err(InvokeError::from_anyhow)?
		.data()
		.into();
	Ok(Response::new(data))
}

#[tauri::command]
pub(crate) async fn storage_set(
	actor_handle: tauri::State<'_, ActorHandle<ApplicationActorMessage>>,
	session: SessionId,
	data: Vec<u8>,
) -> Result<Response, InvokeError> {
	let block = Block::<DefaultParams>::new_data(KnownMultiCodec::Raw, data);
	let cid = actor_handle
		.request(|r| ApplicationActorMessage::StorageSet(session, block, r))
		.await
		.map_err(InvokeError::from_error)?
		.map_err(InvokeError::from_anyhow)?;
	Ok(Response::new(to_cbor(&cid).map_err(InvokeError::from_error)?))
}
