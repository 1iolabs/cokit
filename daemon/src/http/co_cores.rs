use crate::types::http_error::{HttpError, HttpResult};
use axum::{extract::Path, Extension, Json};
use co_sdk::{Application, CoId};
use hyper::StatusCode;
use serde_json::{to_value, Value};

/// CO Core State.
///
/// Method: GET
/// Route: /cos/:id/cores
pub async fn get(
	Path(co_id): Path<CoId>,
	application: Extension<Application>,
) -> HttpResult<(StatusCode, Json<Value>)> {
	let reducer = application
		.co_reducer(&co_id)
		.await?
		.ok_or(HttpError::NotFound(anyhow::anyhow!("Co not found: {}", co_id)))?;
	let state = reducer.co().await?;
	let cores: Vec<_> = state.cores.iter().collect();
	Ok((StatusCode::OK, Json(to_value(cores)?)))
}
