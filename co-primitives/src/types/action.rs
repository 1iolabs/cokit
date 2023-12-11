use crate::{Date, Did};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReducerAction<T> {
	pub from: Did,
	pub time: Date,
	pub payload: T,
}
