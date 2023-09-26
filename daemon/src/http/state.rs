use crate::types::http_error::HttpResult;
use axum::{Extension, Json};
use co_sdk::StoreType;
use hyper::StatusCode;
use serde_json::{to_value, Value};

pub async fn get(store: Extension<StoreType>) -> HttpResult<(StatusCode, Json<Value>)> {
	let state = store.state().await;
	Ok((StatusCode::OK, Json(to_value(state)?)))
}
