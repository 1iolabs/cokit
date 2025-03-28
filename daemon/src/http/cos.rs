use crate::types::http_error::HttpResult;
use axum::{Extension, Json};
use co_sdk::{state::memberships, Application, CreateCo, Did, Tags};
use futures::StreamExt;
use hyper::StatusCode;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum GetItem {
	Ok { id: String, did: Did, tags: Tags },
	Err { err: String },
}

/// Read COs.
///
/// Method: GET
/// Route: /cos
#[axum_macros::debug_handler]
pub async fn get(application: Extension<Application>) -> HttpResult<(StatusCode, Json<Vec<GetItem>>)> {
	let local_co = application.local_co_reducer().await?;
	let memberships: Vec<GetItem> = memberships(local_co.storage(), local_co.reducer_state().await.co())
		.map(|item| -> GetItem {
			match item {
				Ok((id, did, tags, _membership_state)) => GetItem::Ok { id: id.into(), did, tags },
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
pub async fn post(
	application: Extension<Application>,
	Json(payload): Json<Value>,
) -> HttpResult<(StatusCode, Json<Value>)> {
	let body: CreateCo = serde_json::from_value(payload)?;
	let id = body.id.clone().to_string();
	application.create_co(application.local_identity(), body).await?;
	Ok((StatusCode::OK, Json(id.into())))
}
