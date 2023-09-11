use anyhow::{Error, Result};
use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;

pub struct HttpError(Error);

pub type HttpResult<T> = Result<T, HttpError>;

impl IntoResponse for HttpError {
	fn into_response(self) -> axum::response::Response {
		(StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "message": format!("Something went wrong: {}", self.0) })))
			.into_response()
	}
}

impl<E> From<E> for HttpError
where
	E: Into<anyhow::Error>,
{
	fn from(err: E) -> Self {
		Self(err.into())
	}
}
