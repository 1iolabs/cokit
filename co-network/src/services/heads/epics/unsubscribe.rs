use crate::services::{
	heads::{
		actor::{to_topic, to_topic_hash, HeadsContext, HeadsState},
		HeadsAction,
	},
	network::UnsubscribeGossipTask,
};
use co_actor::Actions;
use co_primitives::CoTryStreamExt;
use futures::{FutureExt, Stream};

/// When all subscriptions has been unsubscribed remove gossip subscription.
pub fn unsubscribe(
	_actions: &Actions<HeadsAction, HeadsState, HeadsContext>,
	action: &HeadsAction,
	state: &HeadsState,
	context: &HeadsContext,
) -> Option<impl Stream<Item = Result<HeadsAction, anyhow::Error>> + Send + 'static> {
	match action {
		HeadsAction::Unsubscribe(action) => {
			let action = action.clone();
			let hash = to_topic_hash(&action.network);
			let is_last_unsubscribe = state
				.heads
				.get(&hash)
				.map(|subscriptions| subscriptions.is_empty())
				.unwrap_or(true);
			if is_last_unsubscribe {
				let topic = to_topic(&action.network);
				let context = context.clone();
				Some(
					async move { UnsubscribeGossipTask::unsubscribe(context.network, topic).await }
						.into_stream()
						.try_ignore_elements(),
				)
			} else {
				None
			}
		},
		_ => None,
	}
}
