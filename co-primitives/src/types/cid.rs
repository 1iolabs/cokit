use libipld::Cid;
use schemars::{schema::SchemaObject, JsonSchema};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default, Copy)]
pub struct CoCid(Cid);

impl JsonSchema for CoCid {
	fn schema_name() -> String {
		"Cid".to_owned()
	}

	fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
		let cid_ref = SchemaObject::new_ref("cid.json".to_owned());
		schemars::schema::Schema::Object(cid_ref)
	}
}

impl From<Cid> for CoCid {
	fn from(value: Cid) -> Self {
		CoCid(value)
	}
}
