use co_api::{reduce, DagMap, DagSet, Did, Reducer};
use serde::{Deserialize, Serialize};
use std::cmp::Ord;

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct Roles {
	pub roles: DagMap<Did, DagSet<Role>>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Role {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RoleAction {}

impl Reducer for Roles {
	type Action = RoleAction;

	fn reduce(self, _event: &co_api::ReducerAction<Self::Action>, _context: &mut dyn co_api::Context) -> Self {
		todo!()
	}
}

#[no_mangle]
pub extern "C" fn state() {
	reduce::<Roles>()
}
