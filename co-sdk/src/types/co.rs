use libipld::ipld::Ipld;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const CO_CORE_NAME: &str = "co";

#[deprecated]
pub type CoId = String;

#[deprecated]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Co {
	pub id: CoId,
	pub name: String,
	pub data: BTreeMap<String, Ipld>,
}

#[deprecated]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoCreate {
	pub id: Option<String>,
	pub name: String,
	pub data: Option<BTreeMap<String, Ipld>>,
}

impl Into<Co> for CoCreate {
	fn into(self) -> Co {
		Co {
			id: self.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
			name: self.name,
			data: self.data.unwrap_or_else(|| BTreeMap::new()),
		}
	}
}

#[deprecated]
#[derive(Default, PartialEq, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CoExecuteState {
	#[default]
	Stopped,
	Starting,
	Running,
	Stopping,
}
