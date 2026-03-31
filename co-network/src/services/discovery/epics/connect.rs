// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::services::{
	discovery::{
		action::{DialFailedAction, DiscoveryAction, SendResolveAction},
		actor::DiscoveryContext,
		state::DiscoveryState,
	},
	network::{DialNetworkTask, DidCommSendNetworkTask},
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
	let addresses = dial_action.addresses.clone();
	Some(
		async move {
			let result = DialNetworkTask::dial(&network, Some(peer_id), addresses).await;
			match result {
				Ok(_) => Ok(None),
				Err(err) => {
					tracing::warn!(?err, ?peer_id, "discovery-dial-failed");
					Ok(Some(DiscoveryAction::DialFailed(DialFailedAction { request_id, peer_id })))
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
