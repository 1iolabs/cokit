// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use hyper::StatusCode;
use serde::{Serialize, Serializer};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Reference {
	id: Uuid,
}

impl Default for Reference {
	fn default() -> Self {
		Self::new()
	}
}

impl Reference {
	pub fn new() -> Self {
		Self { id: Uuid::new_v4() }
	}
}

impl PartialEq<Reference> for Reference {
	fn eq(&self, other: &Reference) -> bool {
		self.id == other.id
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct Request<T> {
	pub reference: Reference,
	pub request: T,
}

impl<T> Request<T> {
	pub fn new(request: T) -> Self {
		Self { reference: Reference::new(), request }
	}

	pub fn response<R>(&self, response: Result<R, ResponseError>) -> Response<R> {
		Response { reference: self.reference.clone(), response }
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct Response<T> {
	pub reference: Reference,
	pub response: Result<T, ResponseError>,
}

impl<T> Response<T> {
	pub fn new(reference: Reference, response: Result<T, ResponseError>) -> Self {
		Self { reference, response }
	}
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResponseError {
	pub message: String,
	#[serde(serialize_with = "serialize_status")]
	pub status: Option<StatusCode>,
	pub description: Option<String>,
}

fn serialize_status<S: Serializer>(status: &Option<StatusCode>, s: S) -> Result<S::Ok, S::Error> {
	match status {
		Some(i) => s.serialize_u16(i.as_u16()),
		None => s.serialize_none(),
	}
}

impl ResponseError {
	pub fn with_status(self, status: StatusCode) -> Self {
		Self { status: Some(status), ..self }
	}
}

impl From<anyhow::Error> for ResponseError {
	fn from(val: anyhow::Error) -> Self {
		ResponseError {
			message: format!("{}", &val),
			status: Some(StatusCode::INTERNAL_SERVER_ERROR),
			description: Some(format!("{:?}", &val)),
		}
	}
}
