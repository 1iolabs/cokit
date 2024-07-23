use super::{
	context::{Context, State},
	epic_ext::EpicExt,
};
use crate::{Action, Epic};

mod core_action_push;
mod didcomm_receive;
mod invite_receive;
mod invite_send;
mod join_receive;
mod join_send;
mod key_request_send;

pub fn epic() -> impl Epic<Action, State, Context> + Send + 'static {
	core_action_push::core_action_push
		.with(invite_send::invite_send)
		.with(didcomm_receive::didcomm_receive)
		.with(invite_receive::invite_receive)
		.with(join_send::join_send)
		.with(join_receive::join_receive)
		.with(key_request_send::key_request_send)
}
