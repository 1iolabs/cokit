// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::services::connections::{
	action::{ConnectionAction, DisconnectAction, DisconnectReason, DisconnectedAction},
	actor::ConnectionsContext,
	ConnectionState,
};
use co_actor::{Actions, Epic};
use futures::{stream, Stream};

pub struct DisconnectEpic();
impl DisconnectEpic {
	pub fn new() -> Self {
		Self()
	}
}

fn network_can_disconnect(state: &ConnectionState, network: &co_primitives::Network) -> bool {
	state
		.networks
		.get(network)
		.is_some_and(|connection| connection.references.is_empty() && connection.did_references.is_empty())
}

impl Epic<ConnectionAction, ConnectionState, ConnectionsContext> for DisconnectEpic {
	fn epic(
		&mut self,
		_actions: &Actions<ConnectionAction, ConnectionState, ConnectionsContext>,
		message: &ConnectionAction,
		state: &ConnectionState,
		_context: &ConnectionsContext,
	) -> Option<impl Stream<Item = Result<ConnectionAction, anyhow::Error>> + 'static> {
		match message {
			ConnectionAction::Disconnect(DisconnectAction { network }) if network_can_disconnect(state, network) => {
				// TODO: implement
				Some(stream::iter([Ok(ConnectionAction::Disconnected(DisconnectedAction {
					network: network.clone(),
					reason: DisconnectReason::Close,
				}))]))
			},
			_ => None,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::connections::DidUseAction;
	use co_actor::{time::Instant, Reducer};
	use co_primitives::{Network, NetworkRendezvous};

	#[test]
	fn reacquired_network_cannot_run_a_stale_disconnect() {
		let mut state = ConnectionState::default();
		let network =
			Network::Rendezvous(NetworkRendezvous { namespace: "disconnect-reuse".to_owned(), addresses: vec![] });
		state.reduce(ConnectionAction::DidUse(DidUseAction::new(
			"did:local:alice".to_owned(),
			"did:local:bob".to_owned(),
			Instant::now(),
			[network.clone()].into(),
		)));

		assert!(!network_can_disconnect(&state, &network));
	}
}
