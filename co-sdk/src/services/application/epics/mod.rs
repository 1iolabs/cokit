use super::Action;
use crate::CoContext;
use co_actor::{Epic, MergeEpic, TracingEpic};
use co_primitives::Tags;

mod co_didcomm_send;
mod co_flush_staged;
mod co_heads_publish;
mod co_heads_subscribe;
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
mod membership_update;
mod network_queue;
mod push_heads;

pub fn epic(tags: Tags) -> impl Epic<Action, (), CoContext> + Send + 'static {
	MergeEpic::new()
		.join(core_action_push::core_action_push)
		.join(did_subscribe::keystore_changed)
		.join(did_subscribe::network_started)
		.join(didcomm_receive::didcomm_receive)
		.join(invite_receive::invite_receive)
		.join(invite_send::invite_send)
		.join(invite_send::invite_sent)
		.join(join_receive::join_receive)
		.join(join_send::join_send)
		.join(join_send::join_sent)
		.join(joined::joined)
		.join(joined::joined_fetch)
		.join(heads_message::heads_message_receive)
		.join(heads_message::heads_message_heads)
		.join(heads_message::heads_message_heads_request)
		.join(didcomm_send::didcomm_send)
		.join(key_request_receive::key_request_receive)
		.join(key_request_send::key_request_send)
		.join(membership_update::membership_update)
		.join(membership_update::membership_remove)
		.join(push_heads::PushHeadsEpic::default())
		.join(co_heads_publish::co_heads_publish)
		.join(co_heads_subscribe::CoHeadsSubscribeEpic::default())
		.join(co_flush_staged::CoFlushStagedEpic::default())
		.join(co_didcomm_send::co_didcomm_send)
		.join(network_queue::network_queue_message_epic)
		.join(network_queue::network_started_epic)
		.join(network_queue::NetworkQueueProcessEpic::default())
		.join(TracingEpic::new(tags))
}
