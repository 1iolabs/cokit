use super::context::{Context, State};
use crate::{Action, Epic};

mod core_action_push;

pub fn epic() -> impl Epic<Action, State, Context> + Send + 'static {
	core_action_push::core_action_push
}
