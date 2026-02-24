// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::services::connections::{action::ConnectionAction, actor::ConnectionsContext, ConnectionState};
use co_actor::Actions;
use futures::{FutureExt, Stream};

/// Try to keep [`crate::NetworkSettings::peers_threshold`] open at all time.
pub fn peers_threshold_epic(
	_actions: &Actions<ConnectionAction, ConnectionState, ConnectionsContext>,
	message: &ConnectionAction,
	state: &ConnectionState,
	context: &ConnectionsContext,
) -> Option<impl Stream<Item = Result<ConnectionAction, anyhow::Error>> + 'static> {
	match message {
		ConnectionAction::PeerConnectionClosed(_) => match context.settings.peers_threshold {
			Some(peers_threshold) if count_connected_peers(state) < peers_threshold => {
				Some(async move { Ok(ConnectionAction::InsufficentPeers) }.into_stream())
			},
			_ => None,
		},
		_ => None,
	}
}

fn count_connected_peers(state: &ConnectionState) -> u32 {
	state.peers.iter().fold(
		0u32,
		|count, (_peer, peer_connection)| {
			if peer_connection.connected {
				count + 1
			} else {
				count
			}
		},
	)
}
