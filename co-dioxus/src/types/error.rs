// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

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
