use crate::{Action, CoContext};
use co_actor::{Actions, Epic};
use co_primitives::CoId;
use futures::{stream, Stream, StreamExt};
use std::collections::HashMap;

/// Remember staged actions and dispatch them after next flush.
#[derive(Debug, Clone, Default)]
pub struct CoFlushStagedEpic {
	staged: HashMap<CoId, Vec<Action>>,
}
impl Epic<Action, (), CoContext> for CoFlushStagedEpic {
	fn epic(
		&mut self,
		_actions: &Actions<Action, (), CoContext>,
		action: &Action,
		_state: &(),
		_context: &CoContext,
	) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
		match action {
			Action::CoStaged { co, action } => {
				self.staged.entry(co.clone()).or_default().push(action.as_ref().clone());
				None
			},
			Action::CoFlush { co, info: _ } => Some(stream::iter(self.staged.remove(co).unwrap_or_default()).map(Ok)),
			_ => None,
		}
	}
}
