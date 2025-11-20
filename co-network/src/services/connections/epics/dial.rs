use crate::{
	connections::DialCompletedAction,
	services::{
		connections::{action::ConnectionAction, actor::ConnectionsContext, ConnectionState},
		network::DialNetworkTask,
	},
};
use co_actor::Actions;
use futures::{FutureExt, Stream};
use std::time::Instant;

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
						ok: result.is_ok(),
						time: Instant::now(),
					}))
				}
				.into_stream(),
			)
		},
		_ => None,
	}
}
