// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

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
impl Epic<ConnectionAction, ConnectionState, ConnectionsContext> for DisconnectEpic {
	fn epic(
		&mut self,
		_actions: &Actions<ConnectionAction, ConnectionState, ConnectionsContext>,
		message: &ConnectionAction,
		_state: &ConnectionState,
		_context: &ConnectionsContext,
	) -> Option<impl Stream<Item = Result<ConnectionAction, anyhow::Error>> + 'static> {
		match message {
			ConnectionAction::Disconnect(DisconnectAction { network }) => {
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
