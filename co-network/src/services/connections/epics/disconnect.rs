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
