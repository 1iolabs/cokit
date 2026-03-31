// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

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
