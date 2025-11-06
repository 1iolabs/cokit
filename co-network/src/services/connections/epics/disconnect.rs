use crate::{
	services::connections::{
		ConnectionAction, ConnectionState, DisconnectAction, DisconnectReason, DisconnectedAction,
	},
	CoContext,
};
use co_actor::{Actions, Epic};
use futures::{stream, Stream};

pub struct DisconnectEpic();
impl DisconnectEpic {
	pub fn new() -> Self {
		Self()
	}
}
impl Epic<ConnectionAction, ConnectionState, CoContext> for DisconnectEpic {
	fn epic(
		&mut self,
		_actions: &Actions<ConnectionAction, ConnectionState, CoContext>,
		message: &ConnectionAction,
		_state: &ConnectionState,
		_context: &CoContext,
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
