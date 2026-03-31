// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::services::{
	discovery::{
		action::{DiscoveryAction, GossipSubscribeAction},
		actor::DiscoveryContext,
		state::DiscoveryState,
	},
	network::SubscribeGossipTask,
};
use co_actor::Actions;
use futures::{FutureExt, Stream, StreamExt};
use std::future::ready;

/// Handles `GossipSubscribe` by subscribing to a gossipsub topic.
pub fn did_subscribe_epic(
	_actions: &Actions<DiscoveryAction, DiscoveryState, DiscoveryContext>,
	action: &DiscoveryAction,
	_state: &DiscoveryState,
	context: &DiscoveryContext,
) -> Option<impl Stream<Item = Result<DiscoveryAction, anyhow::Error>> + Send + 'static> {
	match action {
		DiscoveryAction::GossipSubscribe(GossipSubscribeAction { topic }) => {
			let network = context.network.clone();
			let topic = libp2p::gossipsub::IdentTopic::new(topic);
			Some(
				async move {
					let result = SubscribeGossipTask::subscribe(network, topic).await;
					if let Err(err) = &result {
						tracing::warn!(?err, "discovery-gossip-subscribe-failed");
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
