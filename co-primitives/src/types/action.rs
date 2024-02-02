use crate::{Date, Did};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReducerAction<T> {
	/// Sender.
	#[serde(rename = "f")]
	pub from: Did,

	/// Time when the event occured.
	///
	/// Note: The time from the dispatching device is used.
	#[serde(rename = "t")]
	pub time: Date,

	/// COre affected by this action.
	#[serde(rename = "c")]
	pub core: String,

	/// Action payload.
	#[serde(rename = "p")]
	pub payload: T,
}
