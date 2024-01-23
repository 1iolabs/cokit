use crate::{Date, Did};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReducerAction<T> {
	/// Sender.
	pub from: Did,

	/// Time when the event occured.
	///
	/// Note: The time from the dispatching device is used.
	pub time: Date,

	/// COre affected by this action.
	pub core: String,

	/// Action payload.
	pub payload: T,
}
