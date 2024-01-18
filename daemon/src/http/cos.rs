use crate::{library::read_cos::read_cos, types::http_error::HttpResult};
use axum::{Extension, Json};
use co_sdk::{ActionsType, Co, CoAction, CoCreate, CoStorage, Request, StoreType};
use hyper::StatusCode;
use rxrust::prelude::*;
use serde::Serialize;
use serde_json::{to_value, Value};
use std::ops::Deref;
use tokio::join;

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum GetCosItem {
	Ok(Co),
	Err {
		#[serde(rename = "$err")]
		err: String,
	},
}

/// Read COs.
///
/// Method: GET
/// Route: /cos
#[axum_macros::debug_handler]
pub async fn get(
	storage: Extension<CoStorage>,
	store: Extension<StoreType>,
) -> HttpResult<(StatusCode, Json<Vec<GetCosItem>>)> {
	let result: Vec<GetCosItem> = read_cos(&storage, &store.state().await.root)
		.await?
		.into_iter()
		.map::<GetCosItem, _>(|i| match i {
			Ok(c) => GetCosItem::Ok(c),
			Err(e) => GetCosItem::Err { err: format!("{}", e) },
		})
		.collect();
	Ok((StatusCode::OK, Json(result)))
}

/// Create CO.
///
/// Method: POST
/// Route: /cos
#[axum_macros::debug_handler]
pub async fn post(
	store: Extension<StoreType>,
	actions: Extension<ActionsType>,
	Json(payload): Json<Value>,
) -> HttpResult<(StatusCode, Json<Value>)> {
	let actions = actions.deref().clone();

	// parse
	let body: CoCreate = serde_json::from_value(payload)?;

	// create
	let request = Request::new(body);
	let action = CoAction::CoCreate(request.clone());
	let (response, _) = join!(
		actions
			.filter_map(move |action| match action {
				CoAction::CoCreateResponse(response) => {
					if response.reference == request.reference {
						Some(response)
					} else {
						None
					}
				},
				_ => None,
			})
			.take(1)
			.to_future(),
		store.dispatch(action),
	);

	// response
	match response??.response {
		Ok(i) => Ok((StatusCode::OK, Json(to_value(i)?))),
		Err(e) => Ok((e.status.unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), Json(to_value(e)?))),
	}
}
