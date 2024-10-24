use super::Action;
use crate::CoContext;
use co_actor::{Epic, MergeEpic, TracingEpic};
use co_primitives::Tags;

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

pub fn epic(tags: Tags) -> impl Epic<Action, (), CoContext> + Send + 'static {
	MergeEpic::new()
		.join(core_action_push::core_action_push)
		.join(did_subscribe::keystore_changed)
		.join(did_subscribe::network_started)
		.join(didcomm_receive::didcomm_receive)
		.join(invite_receive::invite_receive)
		.join(invite_send::invite_send_action)
		.join(invite_send::invite_send)
		.join(join_receive::join_receive)
		.join(join_send::join_send)
		.join(joined::joined)
		.join(joined::joined_fetch)
		.join(heads_message::heads_message_receive)
		.join(heads_message::heads_message_heads)
		.join(heads_message::heads_message_heads_request)
		.join(didcomm_send::didcomm_send)
		.join(key_request_receive::key_request_receive)
		.join(key_request_send::KeyRequestSend::new())
		.join(TracingEpic::new(tags))
}
