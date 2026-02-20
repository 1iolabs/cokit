use super::Action;
use crate::CoContext;
use co_actor::{Epic, MergeEpic, TracingEpic};
use co_primitives::Tags;

#[cfg(feature = "network")]
mod co_didcomm_send;
#[cfg(feature = "network")]
mod co_heads_publish;
#[cfg(feature = "network")]
mod co_heads_subscribe;
mod core_action_push;
#[cfg(feature = "network")]
mod did_subscribe;
#[cfg(feature = "network")]
mod didcomm_connected;
#[cfg(feature = "network")]
mod didcomm_receive;
#[cfg(feature = "network")]
mod didcomm_send;
#[cfg(feature = "network")]
mod heads_message;
#[cfg(feature = "network")]
mod invite_receive;
#[cfg(feature = "network")]
mod invite_send;
#[cfg(feature = "network")]
mod join_receive;
#[cfg(feature = "network")]
mod join_send;
#[cfg(feature = "network")]
mod joined;
#[cfg(feature = "network")]
mod key_request_receive;
#[cfg(feature = "network")]
mod key_request_send;
#[cfg(feature = "network")]
mod membership_update;
#[cfg(feature = "network")]
mod network_block_get;
#[cfg(feature = "network")]
mod network_queue;
#[cfg(feature = "network")]
mod network_start;
#[cfg(feature = "network")]
mod push_heads;
mod resolve_private_identity;

pub fn epic(tags: Tags) -> impl Epic<Action, (), CoContext> + Send + 'static {
	let epic = MergeEpic::new();

	// epics
	let epic = epic
		.join(core_action_push::core_action_push)
		.join(resolve_private_identity::resolve_private_identity);

	// network epics
	#[cfg(feature = "network")]
	let epic = epic
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
		.join(didcomm_connected::didcomm_connected)
		.join(key_request_receive::key_request_receive)
		.join(key_request_send::key_request_send)
		.join(key_request_send::network_task_execute)
		.join(membership_update::membership_update)
		.join(membership_update::membership_remove)
		.join(push_heads::PushHeadsEpic::default())
		.join(co_heads_publish::co_heads_publish)
		.join(co_heads_subscribe::CoHeadsSubscribeEpic::default())
		.join(co_didcomm_send::co_didcomm_send)
		.join(network_queue::network_queue_message_epic)
		.join(network_queue::network_started_epic)
		.join(network_queue::NetworkQueueProcessEpic::default())
		.join(network_block_get::network_block_get)
		.join(network_block_get::network_task_execute)
		.join(network_start::network_start);

	// trace
	epic.join(TracingEpic::new(tags))
}
