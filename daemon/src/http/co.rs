use crate::types::http_error::HttpResult;
use axum::{extract::Path, Extension, Json};
use co_sdk::{ActionsType, CoAction, CoExecuteState, StoreType};
use hyper::StatusCode;
use rxrust::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::ops::Deref;
use tokio::join;

/// Start/Stop CO.
///
/// Method: POST
/// Route: /cos/:id
#[axum_macros::debug_handler]
pub async fn post(
	Path(co_id): Path<String>,
	store: Extension<StoreType>,
	actions: Extension<ActionsType>,
	Json(payload): Json<Value>,
) -> HttpResult<(StatusCode, Json<Value>)> {
	let actions = actions.deref().clone();

	// parse
	let payload: CoPayload = serde_json::from_value(payload)?;

	// validate
	let state = store.state().await;
	let execute_state = state.execute.get(&co_id);
	match execute_state {
		Some(CoExecuteState::Running) => match payload.type_ {
			CoType::Start => return Ok((StatusCode::CONFLICT, json!({"message": "CO already running."}).into())),
			CoType::Stop => {},
		},
		Some(CoExecuteState::Stopping) =>
			return Ok((StatusCode::CONFLICT, json!({"message": "CO is currently stopping."}).into())),
		Some(CoExecuteState::Starting) =>
			return Ok((StatusCode::CONFLICT, json!({"message": "CO is currently stopping."}).into())),
		Some(CoExecuteState::Stopped) | None => match payload.type_ {
			CoType::Start => {},
			CoType::Stop => return Ok((StatusCode::CONFLICT, json!({"message": "CO already stopped."}).into())),
		},
	}

	match payload.type_ {
		CoType::Start => {
			// start and wait for running of stopped (failed)
			let action = CoAction::CoStartup { id: co_id.clone() };
			let (response, _) = join!(
				actions
					.filter_map(move |action| match action {
						CoAction::CoExecuteStateChanged { id, state: CoExecuteState::Running } if id == co_id => {
							Some(CoExecuteState::Running)
						},
						CoAction::CoExecuteStateChanged { id, state: CoExecuteState::Stopped } if id == co_id => {
							Some(CoExecuteState::Stopped)
						},
						_ => None,
					})
					.take(1)
					.to_future(),
				store.dispatch(action),
			);

			// response
			match response?? {
				CoExecuteState::Running => Ok((StatusCode::OK, json!("{}").into())),
				CoExecuteState::Stopped =>
					Ok((StatusCode::INTERNAL_SERVER_ERROR, json!({"message": "CO startup failed."}).into())),
				_ => unreachable!("Invalid response state"),
			}
		},
		CoType::Stop => {
			// start and wait for running of stopped (failed)
			let action = CoAction::CoShutdown { id: co_id.clone() };
			let (response, _) = join!(
				actions
					.filter_map(move |action| match action {
						CoAction::CoExecuteStateChanged { id, state: CoExecuteState::Stopped } if id == co_id => {
							Some(CoExecuteState::Stopped)
						},
						_ => None,
					})
					.take(1)
					.to_future(),
				store.dispatch(action),
			);

			// response
			match response?? {
				CoExecuteState::Stopped => Ok((StatusCode::OK, json!("{}").into())),
				_ => unreachable!("Invalid response state"),
			}
		},
	}
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum CoType {
	Start,
	Stop,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CoPayload {
	#[serde(rename = "type")]
	pub type_: CoType,
}
