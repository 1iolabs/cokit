// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::services::{
	discovery::{
		action::{DialPeerAction, DiscoveryAction, SendResolveAction},
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
	let DiscoveryAction::DialPeer(DialPeerAction { peer_id, addresses, .. }) = action else {
		return None;
	};
	let network = context.network.clone();
	let peer_id = *peer_id;
	let addresses = addresses.clone();
	Some(
		async move {
			let result = DialNetworkTask::dial(&network, Some(peer_id), addresses).await;
			if let Err(err) = &result {
				tracing::warn!(?err, ?peer_id, "discovery-dial-failed");
			}
			// dial result is handled via PeerConnected/PeerDisconnected swarm events.
			Ok(None)
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
