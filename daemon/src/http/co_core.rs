use crate::types::http_error::{HttpError, HttpResult};
use axum::{extract::Path, Extension, Json};
use co_sdk::{Application, BlockStorageExt, CoId};
use hyper::StatusCode;
use ipld_core::ipld::Ipld;
use serde_json::{json, to_value, Value};

/// CO Core State.
///
/// Method: GET
/// Route: `/cos/:id/cores/:core`
pub async fn get(
	Path((co_id, core)): Path<(CoId, String)>,
	application: Extension<Application>,
) -> HttpResult<(StatusCode, Json<Value>)> {
	let reducer = application
		.co_reducer(&co_id)
		.await?
		.ok_or(HttpError::NotFound(anyhow::anyhow!("Co not found: {}", co_id)))?;
	let (storage, state) = reducer.co().await?;
	let core = state
		.cores
		.get(&core)
		.ok_or(HttpError::NotFound(anyhow::anyhow!("Core not found: {}", core)))?;
	let body = match core.state {
		Some(cid) => {
			let ipld: Ipld = storage.get_deserialized(&cid).await?;
			to_value(ipld)?
		},
		None => json!(null),
	};
	Ok((StatusCode::OK, Json(body)))
}
