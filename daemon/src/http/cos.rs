use crate::types::http_error::HttpResult;
use axum::{Extension, Json};
use co_core_co::Co;
use co_core_membership::Memberships;
use co_sdk::{memberships, CoReducer, Cores, Tags, CO_CORE_CO, CO_CORE_MEMBERSHIP};
use futures::{StreamExt, TryStreamExt};
use hyper::StatusCode;
use libipld::Cid;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum GetItem {
	Ok { id: String, state: Option<Cid>, tags: Tags },
	Err { err: String },
}

/// Read COs.
///
/// Method: GET
/// Route: /cos
#[axum_macros::debug_handler]
pub async fn get(local_co: Extension<CoReducer>) -> HttpResult<(StatusCode, Json<Vec<GetItem>>)> {
	let memberships: Vec<GetItem> = memberships(local_co.0.clone())
		.map(|item| -> GetItem {
			match item {
				Ok((id, state, tags)) => GetItem::Ok { id, state, tags },
				Err(e) => GetItem::Err { err: format!("{:?}", e) },
			}
		})
		.collect()
		.await;
	Ok((StatusCode::OK, Json(memberships)))
}

/// Create CO.
///
/// Method: POST
/// Route: /cos
#[axum_macros::debug_handler]
pub async fn post(Json(_payload): Json<Value>) -> HttpResult<(StatusCode, Json<Value>)> {
	unimplemented!()
}
