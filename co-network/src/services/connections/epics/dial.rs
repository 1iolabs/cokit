// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{
	connections::DialCompletedAction,
	services::{
		connections::{action::ConnectionAction, actor::ConnectionsContext, ConnectionState},
		network::{is_concurrent_dial_rejection, DialNetworkTask},
	},
};
use co_actor::{time, Actions};
use futures::{FutureExt, Stream};

fn dial_completed_ok<T>(result: &Result<T, anyhow::Error>) -> bool {
	result.is_ok() || result.as_ref().err().is_some_and(is_concurrent_dial_rejection)
}

/// Dial a peer.
pub fn dial_epic(
	_actions: &Actions<ConnectionAction, ConnectionState, ConnectionsContext>,
	message: &ConnectionAction,
	_state: &ConnectionState,
	context: &ConnectionsContext,
) -> Option<impl Stream<Item = Result<ConnectionAction, anyhow::Error>> + 'static> {
	match message {
		ConnectionAction::Dial(action) => {
			let context = context.clone();
			let action = action.clone();
			Some(
				async move {
					let result = DialNetworkTask::dial(
						&context.network,
						Some(action.peer_id),
						action.endpoints.iter().cloned().collect(),
					)
					.await;
					Ok(ConnectionAction::DialCompleted(DialCompletedAction {
						peer_id: action.peer_id,
						ok: dial_completed_ok(&result),
						time: time::Instant::now(),
					}))
				}
				.into_stream(),
			)
		},
		_ => None,
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use libp2p::swarm::{dial_opts::PeerCondition, DialError};

	#[test]
	fn concurrent_dial_rejection_is_non_failure() {
		let result: Result<(), anyhow::Error> =
			Err(DialError::DialPeerConditionFalse(PeerCondition::DisconnectedAndNotDialing).into());

		assert!(dial_completed_ok(&result));
	}
}
