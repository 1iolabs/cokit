use super::{action::UseAction, ConnectionAction, PeersChangedAction};
use crate::actor::{ActorError, ActorHandle, ResponseStream};
use co_primitives::{CoId, Did, Network};
use futures::Stream;
use std::time::Instant;

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
impl ConnectionMessage {
	pub fn co_use(
		actor: ActorHandle<Self>,
		id: CoId,
		from: Did,
		networks: impl IntoIterator<Item = Network>,
	) -> impl Stream<Item = Result<PeersChangedAction, ActorError>> {
		let action = UseAction { id, from, time: Instant::now(), networks: networks.into_iter().collect() };
		actor.stream(|response| Self::Use(action, response))
	}
}
