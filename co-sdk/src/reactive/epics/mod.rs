use co_participant_invite_send::co_participant_invite_send;
use core_action_push::core_action_push;
use didcomm_receive::didcomm_receive;
use invite_receive::invite_receive;

use super::{
	context::{Context, State},
	epic_ext::EpicExt,
};
use crate::{Action, Epic};

mod co_participant_invite_send;
mod core_action_push;
mod didcomm_receive;
mod invite_receive;

pub fn epic() -> impl Epic<Action, State, Context> + Send + 'static {
	core_action_push
		.with(co_participant_invite_send)
		.with(didcomm_receive)
		.with(invite_receive)
	core_action_push::core_action_push.with(co_participant_invite_send::co_participant_invite_send)
}
