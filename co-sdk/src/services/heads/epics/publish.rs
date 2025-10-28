use crate::{
	network::PublishGossipTask,
	services::heads::{
		actor::{to_topic_hash, HeadsContext, HeadsState},
		HeadsAction,
	},
	types::message::heads::HeadsMessage,
};
use co_actor::Actions;
use co_primitives::{to_cbor, CoTryStreamExt, WeakCid};
use futures::{FutureExt, Stream};

/// Publish new heads gossip.
pub fn publish(
	_actions: &Actions<HeadsAction, HeadsState, HeadsContext>,
	action: &HeadsAction,
	_state: &HeadsState,
	context: &HeadsContext,
) -> Option<impl Stream<Item = Result<HeadsAction, anyhow::Error>> + Send + 'static> {
	match action {
		HeadsAction::Publish(action) => {
			let network = context.network.clone();
			let action = action.clone();
			Some(
				async move {
					let topic = to_topic_hash(&action.network);
					let message = HeadsMessage::Heads(
						action.network.id.clone(),
						action.heads.iter().map(WeakCid::from).collect(),
					);
					let data = to_cbor(&message)?;
					PublishGossipTask::publish(network, topic, data).await
				}
				.into_stream()
				.try_ignore_elements(),
			)
		},
		_ => None,
	}
}
