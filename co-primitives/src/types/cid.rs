// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use cid::Cid;
use schemars::{
	schema::{InstanceType, Metadata, ObjectValidation, Schema, SchemaObject, SingleOrVec},
	JsonSchema, Map, Set,
};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Serialize, Deserialize, Hash, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Default, Copy)]
#[serde(into = "Cid", from = "Cid")]
#[repr(transparent)]
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
impl From<CoCid> for Cid {
	fn from(value: CoCid) -> Self {
		value.0
	}
}
impl AsRef<Cid> for CoCid {
	fn as_ref(&self) -> &Cid {
		&self.0
	}
}

#[cfg(test)]
mod tests {
	use super::CoCid;
	use crate::BlockSerializer;

	#[test]
	fn test_serialize() {
		let (cid, _block) = BlockSerializer::default().serialize(&"hello world").unwrap().into_inner();
		let co_cid = CoCid::from(cid);
		let json = serde_ipld_dagjson::to_vec(&co_cid).unwrap();
		assert_eq!(
			std::str::from_utf8(&json).unwrap(),
			"{\"/\":\"bafyr4idksef7ir3qqvc5ilbddm4s3v3da7uedc6k4odiejquu5q3dh2i7e\"}"
		);
	}

	#[test]
	fn test_deserialize() {
		let (cid, _block) = BlockSerializer::default().serialize(&"hello world").unwrap().into_inner();
		let co_cid = CoCid::from(cid);
		let json = serde_ipld_dagjson::to_vec(&co_cid).unwrap();
		let co_cid_deserialize: CoCid = serde_ipld_dagjson::from_slice(&json).unwrap();
		assert_eq!(co_cid_deserialize, co_cid);
	}
}
