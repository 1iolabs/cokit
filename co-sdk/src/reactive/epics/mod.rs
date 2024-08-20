use super::{
	context::{Context, State},
	epic_ext::EpicExt,
};
use crate::{Action, Epic};

mod core_action_push;
mod did_subscribe;
mod didcomm_receive;
mod didcomm_send;
mod heads_message;
mod invite_receive;
mod invite_send;
mod join_receive;
mod join_send;
mod joined;
mod key_request_receive;
mod key_request_send;
mod log;

pub fn epic() -> impl Epic<Action, State, Context> + Send + 'static {
	log::log
		.with(core_action_push::core_action_push)
		.with(did_subscribe::keystore_changed)
		.with(did_subscribe::network_started)
		.with(didcomm_receive::didcomm_receive)
		.with(invite_receive::invite_receive)
		.with(invite_send::invite_send_action)
		.with(invite_send::invite_send)
		.with(join_receive::join_receive)
		.with(join_send::join_send)
		.with(joined::joined)
		.with(joined::joined_fetch)
		.with(key_request_receive::key_request_receive)
		.with(key_request_send::key_request_send)
		.with(heads_message::heads_message_receive)
		.with(heads_message::heads_message_heads)
		.with(heads_message::heads_message_heads_request)
		.with(didcomm_send::didcomm_send)
}
