use super::{action::UseAction, ConnectionAction, PeersChangedAction};
use crate::actor::ResponseStream;

#[derive(Debug)]
pub enum ConnectionMessage {
	/// Use a CO by utilitsing the specified networks.
	Use(UseAction, ResponseStream<PeersChangedAction>),

	/// Action.
	Action(ConnectionAction),
}
impl From<ConnectionAction> for ConnectionMessage {
	fn from(value: ConnectionAction) -> Self {
		Self::Action(value)
	}
}
