// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::library::application_actor::{ApplicationActorMessage, GetActionsRequest};
use anyhow::anyhow;
use co_actor::ActorHandle;
use co_sdk::{from_cbor, to_cbor};
use tauri::ipc::InvokeError;

#[tauri::command]
pub(crate) async fn get_actions(
	actor_handle: tauri::State<'_, ActorHandle<ApplicationActorMessage>>,
	request: tauri::ipc::Request<'_>,
) -> Result<tauri::ipc::Response, InvokeError> {
	let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
		return Err(InvokeError::from_anyhow(anyhow!("Request body must be raw")));
	};
	let body: GetActionsRequest = from_cbor(bytes).map_err(InvokeError::from_error)?;
	let result = actor_handle
		.request(|r| ApplicationActorMessage::GetActions(body, r))
		.await
		.map_err(InvokeError::from_error)?
		.map_err(InvokeError::from_anyhow)?;
	Ok(tauri::ipc::Response::new(to_cbor(&result).map_err(InvokeError::from_error)?))
}
