use derive_more::{From, Into};
use ordered_float::OrderedFloat;
use schemars::{
	gen::SchemaGenerator,
	schema::{InstanceType, Schema, SchemaObject},
	JsonSchema,
};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// f64 float wich uses total order from IEEE 754 (2008 revision).
#[derive(Debug, Clone, Copy, Hash, From, Into, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(from = "f64", into = "f64")]
#[repr(transparent)]
pub struct TotalFloat64(OrderedFloat<f64>);
impl TotalFloat64 {
	pub fn value(&self) -> f64 {
		self.0 .0
	}
}
impl JsonSchema for TotalFloat64 {
	fn schema_name() -> String {
		"double".to_owned()
	}

	fn json_schema(_: &mut SchemaGenerator) -> Schema {
		SchemaObject {
			instance_type: Some(InstanceType::Number.into()),
			format: Some("double".to_owned()),
			..Default::default()
		}
		.into()
	}
}
impl From<TotalFloat64> for f64 {
	fn from(value: TotalFloat64) -> Self {
		value.0.into()
	}
}
impl From<f64> for TotalFloat64 {
	fn from(value: f64) -> Self {
		TotalFloat64(OrderedFloat(value))
	}
}
impl AsRef<f64> for TotalFloat64 {
	fn as_ref(&self) -> &f64 {
		&self.0 .0
	}
}
impl Display for TotalFloat64 {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", AsRef::<f64>::as_ref(self))
	}
}
