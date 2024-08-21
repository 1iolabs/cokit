use crate::{
	reactive::context::{ActionObservable, StateObservable},
	Action, CoContext,
};
use futures::{Stream, StreamExt};
use std::future::ready;

pub fn log(
	actions: ActionObservable,
	_states: StateObservable,
	_context: CoContext,
) -> impl Stream<Item = Action> + Send + 'static {
	actions.filter(|action| {
		tracing::info!(action = ?action, "action");
		ready(false)
	})
}
