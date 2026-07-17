// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::services::{
	discovery::{
		action::{DialFailedAction, DiscoveryAction, SendResolveAction},
		actor::DiscoveryContext,
		state::DiscoveryState,
	},
	network::{is_concurrent_dial_rejection, DialIntent, DialNetworkTask, DidCommSendNetworkTask},
};
use co_actor::Actions;
use futures::{FutureExt, Stream, StreamExt};
use std::{future::ready, time::Duration};

/// Handles `DialPeer` actions.
pub fn dial_epic(
	_actions: &Actions<DiscoveryAction, DiscoveryState, DiscoveryContext>,
	action: &DiscoveryAction,
	_state: &DiscoveryState,
	context: &DiscoveryContext,
) -> Option<impl Stream<Item = Result<DiscoveryAction, anyhow::Error>> + Send + 'static> {
	let DiscoveryAction::DialPeer(dial_action) = action else {
		return None;
	};
	let network = context.network.clone();
	let peer_id = dial_action.peer_id;
	let request_id = dial_action.request_id;
	let intent = dial_action.intent;
	let addresses = dial_action.addresses.clone();
	Some(
		async move {
			let result = DialNetworkTask::dial_with_intent(&network, Some(peer_id), addresses.clone(), intent).await;
			match result {
				Ok(_) => Ok(None),
				Err(err) if intent == DialIntent::Reachability && is_concurrent_dial_rejection(&err) => {
					tracing::debug!(
						?err,
						?peer_id,
						?addresses,
						?intent,
						outcome = "skipped",
						"discovery-dial-skipped-another-dial-in-progress"
					);
					Ok(None)
				},
				Err(err) if intent == DialIntent::Reachability => {
					tracing::warn!(?err, ?peer_id, ?addresses, ?intent, outcome = "failed", "discovery-dial-failed");
					Ok(Some(DiscoveryAction::DialFailed(DialFailedAction { request_id, peer_id })))
				},
				Err(err) => {
					tracing::debug!(
						?err,
						?peer_id,
						?addresses,
						?intent,
						outcome = "failed",
						"discovery-detached-dial-failed"
					);
					Ok(None)
				},
			}
		}
		.into_stream()
		.filter_map(|result: Result<Option<DiscoveryAction>, anyhow::Error>| ready(result.transpose())),
	)
}

/// Handles `SendResolve` actions (send DIDComm resolve response).
pub fn send_resolve_epic(
	_actions: &Actions<DiscoveryAction, DiscoveryState, DiscoveryContext>,
	action: &DiscoveryAction,
	_state: &DiscoveryState,
	context: &DiscoveryContext,
) -> Option<impl Stream<Item = Result<DiscoveryAction, anyhow::Error>> + Send + 'static> {
	let DiscoveryAction::SendResolve(SendResolveAction { from_peer, response, .. }) = action else {
		return None;
	};
	let network = context.network.clone();
	let from_peer = *from_peer;
	let response = response.clone();
	Some(
		async move {
			let result =
				DidCommSendNetworkTask::send(network, [from_peer], response.into(), Duration::from_secs(10)).await;
			if let Err(err) = &result {
				tracing::warn!(?err, ?from_peer, "discovery-send-resolve-failed");
			}
			Ok(None)
		}
		.into_stream()
		.filter_map(|r: Result<Option<DiscoveryAction>, anyhow::Error>| async move {
			match r {
				Ok(Some(a)) => Some(Ok(a)),
				Ok(None) => None,
				Err(e) => Some(Err(e)),
			}
		}),
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::services::{
		discovery::action::DialPeerAction,
		network::{CoNetworkTaskSpawner, DialIntent},
	};
	use co_actor::{Actions, TaskSpawner};
	use co_identity::{IdentityResolver, MemoryIdentityResolver};
	use co_primitives::{CoDate, StaticCoDate};
	use futures::TryStreamExt;
	use libp2p::{
		swarm::{dial_opts::PeerCondition, DialError},
		PeerId,
	};
	use tokio::time::timeout;

	fn test_context(local_peer_id: PeerId) -> DiscoveryContext {
		DiscoveryContext {
			tasks: TaskSpawner::default(),
			network: CoNetworkTaskSpawner::new_closed(local_peer_id),
			date: StaticCoDate(0).boxed(),
			resolver: MemoryIdentityResolver::default().boxed(),
			local_peer_id,
			dial_redundancy: true,
		}
	}

	fn test_state(local_peer_id: PeerId) -> DiscoveryState {
		DiscoveryState {
			local_peer_id,
			dial_redundancy: true,
			next_id: 1,
			requests: Default::default(),
			did_subscriptions: Default::default(),
			pending_discovery: Default::default(),
			timeout: Duration::from_secs(10),
			max_peers: None,
			connected_peers: Default::default(),
			did_peer_cache: Default::default(),
		}
	}

	async fn collect_dial_actions(request_id: Option<u64>, intent: DialIntent) -> (PeerId, Vec<DiscoveryAction>) {
		let local_peer_id = PeerId::random();
		let remote_peer_id = PeerId::random();
		let actions = Actions::default();
		let state = test_state(local_peer_id);
		let context = test_context(local_peer_id);
		let action = DiscoveryAction::DialPeer(DialPeerAction {
			request_id,
			peer_id: remote_peer_id,
			addresses: vec!["/ip4/127.0.0.1/tcp/1".parse().unwrap()],
			intent,
		});

		let stream = dial_epic(&actions, &action, &state, &context).expect("DialPeer should start the dial epic");
		let emitted = timeout(Duration::from_secs(1), stream.try_collect::<Vec<_>>())
			.await
			.expect("dial epic should complete within one second")
			.expect("dial epic should not return a stream error");
		(remote_peer_id, emitted)
	}

	fn assert_dial_failed(actions: &[DiscoveryAction], request_id: Option<u64>, peer_id: PeerId) {
		assert_eq!(actions.len(), 1);
		let DiscoveryAction::DialFailed(failure) = &actions[0] else {
			panic!("expected DialFailed, got {:?}", actions[0]);
		};
		assert_eq!(failure.request_id, request_id);
		assert_eq!(failure.peer_id, peer_id);
	}

	#[test]
	fn reachability_peer_condition_rejection_is_nonfatal() {
		let err = anyhow::Error::from(DialError::DialPeerConditionFalse(PeerCondition::DisconnectedAndNotDialing));

		assert!(is_concurrent_dial_rejection(&err));
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn reachability_failure_emits_request_bound_dial_failed() {
		let (peer_id, actions) = collect_dial_actions(Some(7), DialIntent::Reachability).await;
		assert_dial_failed(&actions, Some(7), peer_id);
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn detached_reachability_failure_preserves_dial_failed() {
		let (peer_id, actions) = collect_dial_actions(None, DialIntent::Reachability).await;
		assert_dial_failed(&actions, None, peer_id);
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn redundancy_failure_does_not_emit_dial_failed() {
		let (_peer_id, actions) = collect_dial_actions(None, DialIntent::Redundancy).await;
		assert!(actions.is_empty());
	}
}
