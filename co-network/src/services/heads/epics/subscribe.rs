// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::services::{
	heads::{
		actor::{to_topic, to_topic_hash, HeadsContext, HeadsState},
		HeadsAction,
	},
	network::SubscribeGossipTask,
};
use co_actor::Actions;
use co_primitives::CoTryStreamExt;
use futures::{FutureExt, Stream};

/// When a topic has the frist subscription subsribe to gossip subscription.
pub fn subscribe(
	_actions: &Actions<HeadsAction, HeadsState, HeadsContext>,
	action: &HeadsAction,
	state: &HeadsState,
	context: &HeadsContext,
) -> Option<impl Stream<Item = Result<HeadsAction, anyhow::Error>> + Send + 'static> {
	match action {
		HeadsAction::Subscribe(action) => {
			let action = action.clone();
			let hash = to_topic_hash(&action.network);
			let is_first_subscribe = state
				.heads
				.get(&hash)
				.map(|subscriptions| subscriptions.len() == 1)
				.unwrap_or(false);
			if is_first_subscribe {
				let topic = to_topic(&action.network);
				let context = context.clone();
				Some(
					async move { SubscribeGossipTask::subscribe(context.network.clone(), topic.clone()).await }
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
