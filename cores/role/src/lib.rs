use co_api::{DagMap, DagSet, Did, Tags};
use serde::{Deserialize, Serialize};
use std::cmp::Ord;

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct Roles {
	pub roles: DagMap<Did, DagSet<Role>>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Role {}
