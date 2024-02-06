use crate::types::http_error::HttpResult;
use axum::{Extension, Json};
use co_core_membership::Memberships;
use co_sdk::{CoReducer, Cores, Tags, CO_CORE_MEMBERSHIP};
use hyper::StatusCode;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum GetItem {
	Ok { id: String, tags: Tags },
}

/// Read COs.
///
/// Method: GET
/// Route: /cos
#[axum_macros::debug_handler]
pub async fn get(local_co: Extension<CoReducer>) -> HttpResult<(StatusCode, Json<Vec<GetItem>>)> {
	let memberships: Memberships = local_co.state(Cores::to_core_name(CO_CORE_MEMBERSHIP)).await?;
	let result: Vec<GetItem> = memberships
		.memberships
		.into_iter()
		.map(|item| GetItem::Ok { id: item.id, tags: item.tags })
		.collect();
	Ok((StatusCode::OK, Json(result)))
}

/// Create CO.
///
/// Method: POST
/// Route: /cos
#[axum_macros::debug_handler]
pub async fn post(Json(_payload): Json<Value>) -> HttpResult<(StatusCode, Json<Value>)> {
	unimplemented!()
}
