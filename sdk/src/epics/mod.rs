use crate::types::{action::CoAction, context::CoContext, state::CoState};
use co_state::{CombineEpics, Reducer};
use std::sync::Arc;

pub mod co_create;
pub mod co_execute;
pub mod initialize;
pub mod store_settings;

pub fn sdk_epics<R>() -> CombineEpics<R, Arc<CoContext>>
where
	R: Reducer<State = CoState, Action = CoAction>,
{
	let mut result = CombineEpics::new();
	result.add(initialize::initialize);
	result.add(co_create::co_create);
	result.add(store_settings::store_settings);
	result.add(co_execute::co_execute);
	result
}
