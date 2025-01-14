use cid::Cid;
use schemars::{
	schema::{InstanceType, Metadata, ObjectValidation, Schema, SchemaObject, SingleOrVec},
	JsonSchema, Map, Set,
};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Default, Copy)]
pub struct CoCid(Cid);
impl Display for CoCid {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(f)
	}
}
impl JsonSchema for CoCid {
	fn schema_name() -> String {
		"Cid".to_owned()
	}

	fn json_schema(_gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
		// cid only has one property which is a slash-key string
		let mut properties = Map::new();
		properties.insert(
			"/".to_owned(),
			Schema::Object(SchemaObject {
				instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::String))),
				..Default::default()
			}),
		);
		// set property as required
		let mut required = Set::new();
		required.insert("/".to_owned());
		// create schema
		let cid_schema = SchemaObject {
			// metadata containing title
			metadata: Some(Box::new(Metadata { title: Some("Cid".to_owned()), ..Default::default() })),
			// sets type to 'object'
			instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::Object))),
			// inserts the 'properties' and 'required' props
			object: Some(Box::new(ObjectValidation { properties, required, ..Default::default() })),
			..Default::default()
		};
		Schema::Object(cid_schema)
	}
}

impl From<Cid> for CoCid {
	fn from(value: Cid) -> Self {
		CoCid(value)
	}
}
impl Into<Cid> for CoCid {
	fn into(self) -> Cid {
		self.0
	}
}
impl AsRef<Cid> for CoCid {
	fn as_ref(&self) -> &Cid {
		&self.0
	}
}
