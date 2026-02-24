// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::library::application_actor::{ApplicationActorMessage, SessionId};
use anyhow::anyhow;
use cid::Cid;
use co_actor::ActorHandle;
use co_primitives::Block;
use co_sdk::{from_cbor, to_cbor};
use serde::Deserialize;
use tauri::ipc::{InvokeError, Request, Response};

#[derive(Debug, Deserialize)]
pub struct StorageGetBody {
	session: SessionId,
	cid: Cid,
}

#[tauri::command]
pub(crate) async fn storage_get(
	actor_handle: tauri::State<'_, ActorHandle<ApplicationActorMessage>>,
	request: Request<'_>,
) -> Result<Response, InvokeError> {
	let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
		return Err(InvokeError::from_anyhow(anyhow!("Request body must be raw")));
	};
	let body: StorageGetBody = from_cbor(bytes).map_err(InvokeError::from_error)?;
	let data: Vec<u8> = actor_handle
		.request(|r| ApplicationActorMessage::StorageGet(body.session, body.cid, r))
		.await
		.map_err(InvokeError::from_error)?
		.map_err(InvokeError::from_anyhow)?
		.data()
		.into();
	Ok(Response::new(data))
}

#[derive(Debug, Deserialize)]
pub struct StorageSetBody {
	session: SessionId,
	cid: Cid,
	#[serde(with = "serde_bytes")]
	data: Vec<u8>,
}

#[tauri::command]
pub(crate) async fn storage_set(
	actor_handle: tauri::State<'_, ActorHandle<ApplicationActorMessage>>,
	request: Request<'_>,
) -> Result<Response, InvokeError> {
	let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
		return Err(InvokeError::from_anyhow(anyhow!("Request body must be raw")));
	};
	let body: StorageSetBody = from_cbor(bytes).map_err(InvokeError::from_error)?;
	tracing::info!(
		"tauri command storage set: \n\tSession: {:#?}\n\tdata: {:#?} \n\tcid:{:#?}",
		body.session,
		body.data,
		body.cid
	);
	let block = Block::new(body.cid, body.data).map_err(InvokeError::from_error)?;
	let cid = actor_handle
		.request(|r| ApplicationActorMessage::StorageSet(body.session, block, r))
		.await
		.map_err(InvokeError::from_error)?
		.map_err(InvokeError::from_anyhow)?;
	Ok(Response::new(to_cbor(&cid).map_err(InvokeError::from_error)?))
}
