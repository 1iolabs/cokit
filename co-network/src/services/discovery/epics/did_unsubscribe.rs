// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::services::{
	discovery::{
		action::{DiscoveryAction, GossipUnsubscribeAction},
		actor::DiscoveryContext,
		state::DiscoveryState,
	},
	network::UnsubscribeGossipTask,
};
use co_actor::Actions;
use futures::{FutureExt, Stream, StreamExt};
use std::future::ready;

/// Handles `GossipUnsubscribe` by unsubscribing from a gossipsub topic.
pub fn did_unsubscribe_epic(
	_actions: &Actions<DiscoveryAction, DiscoveryState, DiscoveryContext>,
	action: &DiscoveryAction,
	_state: &DiscoveryState,
	context: &DiscoveryContext,
) -> Option<impl Stream<Item = Result<DiscoveryAction, anyhow::Error>> + Send + 'static> {
	match action {
		DiscoveryAction::GossipUnsubscribe(GossipUnsubscribeAction { topic }) => {
			let network = context.network.clone();
			let topic = libp2p::gossipsub::IdentTopic::new(topic);
			Some(
				async move {
					let result = UnsubscribeGossipTask::unsubscribe(network, topic).await;
					if let Err(err) = &result {
						tracing::warn!(?err, "discovery-gossip-unsubscribe-failed");
					}
					Ok(None)
				}
				.into_stream()
				.filter_map(|result: Result<Option<DiscoveryAction>, anyhow::Error>| ready(result.transpose())),
			)
		},
		_ => None,
	}
}
