// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::services::{
	discovery::{
		action::{DidPublishAction, DidPublishPendingAction, DiscoveryAction},
		actor::DiscoveryContext,
		state::{did_discovery_topic, DiscoveryState},
	},
	network::{PublishGossipTask, PublishGossipTaskError},
};
use co_actor::Actions;
use futures::{FutureExt, Stream, StreamExt};
use libp2p::gossipsub;
use std::future::ready;

/// Handles `DidPublish` by publishing a DID discovery message to gossipsub.
/// If no peers are subscribed, dispatches `DidPublishPending`.
pub fn did_publish_epic(
	_actions: &Actions<DiscoveryAction, DiscoveryState, DiscoveryContext>,
	action: &DiscoveryAction,
	_state: &DiscoveryState,
	context: &DiscoveryContext,
) -> Option<impl Stream<Item = Result<DiscoveryAction, anyhow::Error>> + Send + 'static> {
	match action {
		DiscoveryAction::DidPublish(DidPublishAction { request_id, discovery }) => {
			let network = context.network.clone();
			let request_id = *request_id;
			let discovery = discovery.clone();
			let topic = did_discovery_topic(&discovery.network);
			let topic_hash = topic.hash();
			let message = discovery.message.clone().into_bytes();
			Some(
				async move {
					match PublishGossipTask::publish(network, topic_hash.clone(), message).await {
						Ok(_) => {
							tracing::trace!(network = ?discovery.network, "discovery-did-published");
							Ok(None)
						},
						Err(PublishGossipTaskError::Gossip(gossipsub::PublishError::NoPeersSubscribedToTopic)) => {
							// we try again when a peer subscribes
							tracing::trace!(
								network = ?discovery.network,
								"discovery-did-pending-no-peers-subscribed"
							);
							Ok(Some(DiscoveryAction::DidPublishPending(DidPublishPendingAction {
								request_id,
								topic: topic_hash,
								discovery,
							})))
						},
						Err(err) => {
							tracing::warn!(?err, network = ?discovery.network, "discovery-did-publish-failed");
							Err(anyhow::Error::from(err))
						},
					}
				}
				.into_stream()
				.filter_map(|result: Result<Option<DiscoveryAction>, anyhow::Error>| ready(result.transpose())),
			)
		},
		_ => None,
	}
}
