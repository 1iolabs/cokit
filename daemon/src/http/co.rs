use crate::types::http_error::HttpResult;
use axum::{extract::Path, Json};
use hyper::StatusCode;
use serde_json::Value;

/// Start/Stop CO.
///
/// Method: POST
/// Route: /cos/:id
#[axum_macros::debug_handler]
pub async fn post(Path(_co_id): Path<String>, Json(_payload): Json<Value>) -> HttpResult<(StatusCode, Json<Value>)> {
	unimplemented!()
}
