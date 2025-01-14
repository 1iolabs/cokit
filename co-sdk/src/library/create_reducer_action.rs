use co_primitives::{Did, ReducerAction};
use ipld_core::{ipld::Ipld, serde::to_ipld};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

/// Utility to create reducer actions.
pub fn create_reducer_action(
	from: impl Into<Did>,
	core: impl Into<String>,
	payload: impl Serialize,
) -> Result<ReducerAction<Ipld>, anyhow::Error> {
	Ok(ReducerAction {
		core: core.into(),
		from: from.into(),
		time: SystemTime::now().duration_since(UNIX_EPOCH).expect("Valid time").as_millis(),
		payload: to_ipld(payload)?,
	})
}
