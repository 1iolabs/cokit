use super::{action::UseAction, ConnectionAction};
use crate::actor::ResponseStream;
use libp2p::PeerId;
use std::collections::BTreeSet;

#[derive(Debug)]
pub enum ConnectionMessage {
	/// Use a CO by utilitsing the specified networks.
	Use(UseAction, ResponseStream<BTreeSet<PeerId>>),

	/// Action.
	Action(ConnectionAction),
}
impl From<ConnectionAction> for ConnectionMessage {
	fn from(value: ConnectionAction) -> Self {
		Self::Action(value)
	}
}
