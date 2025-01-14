use derive_more::{From, Into};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// f64 float wich uses total order from IEEE 754 (2008 revision).
#[derive(Debug, Clone, Copy, From, Into, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct TotalFloat64(pub f64);
impl PartialEq for TotalFloat64 {
	fn eq(&self, other: &Self) -> bool {
		self.0.total_cmp(&other.0) == Ordering::Equal
	}
}
impl Eq for TotalFloat64 {}
impl PartialOrd for TotalFloat64 {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(std::cmp::Ord::cmp(self, other))
	}
}
impl Ord for TotalFloat64 {
	fn cmp(&self, other: &Self) -> Ordering {
		self.0.total_cmp(&other.0)
	}
}
