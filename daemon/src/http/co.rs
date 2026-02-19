// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::types::http_error::{HttpError, HttpResult};
use axum::{extract::Path, Extension, Json};
use co_sdk::{Application, CoId};
use hyper::StatusCode;
use serde_json::{to_value, Value};

/// CO State.
///
/// Method: GET
/// Route: /cos/:id
pub async fn get(
	Path(co_id): Path<CoId>,
	application: Extension<Application>,
) -> HttpResult<(StatusCode, Json<Value>)> {
	let reducer = application
		.co_reducer(&co_id)
		.await?
		.ok_or(HttpError::NotFound(anyhow::anyhow!("Co not found: {}", co_id)))?;
	let (_storage, state) = reducer.co().await?;
	Ok((StatusCode::OK, Json(to_value(state)?)))
}

/// Push Event.
///
/// Method: POST
/// Route: /cos/:id
#[axum_macros::debug_handler]
pub async fn post(Path(_co_id): Path<String>, Json(_payload): Json<Value>) -> HttpResult<(StatusCode, Json<Value>)> {
	unimplemented!()
}
