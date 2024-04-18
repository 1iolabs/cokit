use dioxus::signals::SyncSignal;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};

pub type CoErrorSignal = SyncSignal<Vec<CoError>>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoError {
	id: String,
	// name: String,
	message: String,
	details: String,
	// tags: Tags,
}
impl CoError {
	pub fn from_error<E>(error: E) -> Self
	where
		E: Display + Debug,
	{
		Self { id: uuid::Uuid::new_v4().to_string(), message: format!("{}", error), details: format!("{:?}", error) }
	}
}
