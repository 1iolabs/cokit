use anyhow::Result;
use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;

pub type HttpResult<T> = Result<T, HttpError>;

/// Http Error.
///
/// Note: This not implements std::error::Error on purpose as its only an wrapper for the http API.
/// If we woudl implement std::error::Error we could not use anyhow::Error as source as we can not implement the
/// generic From<E>. Because anyhow::Error implements std::error::Error and T into T is implements in core.
#[derive(Debug)]
pub enum HttpError {
	InternalServerError(anyhow::Error),
	NotFound(anyhow::Error),
}
impl std::fmt::Display for HttpError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			HttpError::InternalServerError(_) => write!(f, "Internal Server Error"),
			HttpError::NotFound(_) => write!(f, "Not Found"),
		}
	}
}
impl<E> From<E> for HttpError
where
	E: Into<anyhow::Error>,
{
	fn from(value: E) -> Self {
		HttpError::InternalServerError(value.into())
	}
}
impl Into<(StatusCode, Option<anyhow::Error>)> for HttpError {
	fn into(self) -> (StatusCode, Option<anyhow::Error>) {
		match self {
			HttpError::InternalServerError(err) => (StatusCode::INTERNAL_SERVER_ERROR, Some(err)),
			HttpError::NotFound(err) => (StatusCode::NOT_FOUND, Some(err)),
		}
	}
}
impl IntoResponse for HttpError {
	fn into_response(self) -> axum::response::Response {
		let message = format!("{}", &self);
		let (code, err) = Into::<(StatusCode, Option<anyhow::Error>)>::into(self);
		let body = if let Some(err) = err {
			Json(json!({ "message": format!("{}", err) }))
		} else {
			Json(json!({ "message": message }))
		};
		(code, body).into_response()
	}
}
