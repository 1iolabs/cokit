use libipld::Cid;
use schemars::{schema::SchemaObject, JsonSchema};
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
		let cid_ref = SchemaObject::new_ref("cid.json".to_owned());
		schemars::schema::Schema::Object(cid_ref)
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
