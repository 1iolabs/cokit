use super::{
	context::{Context, State},
	epic_ext::EpicExt,
};
use crate::{Action, Epic};

mod co_participant_invite_send;
mod core_action_push;

pub fn epic() -> impl Epic<Action, State, Context> + Send + 'static {
	core_action_push::core_action_push.with(co_participant_invite_send::co_participant_invite_send)
}
