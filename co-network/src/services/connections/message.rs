// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use super::action::{ConnectionAction, PeersChangedAction, UseAction};
use crate::compat::Instant;
use co_actor::{ActorError, ActorHandle, ResponseStream};
use co_primitives::{CoId, Did, Network};
use futures::Stream;

#[derive(Debug)]
pub enum ConnectionMessage {
	/// Use a CO by utilizing the specified networks.
	Use(UseAction, ResponseStream<PeersChangedAction>),

	/// Action.
	Action(ConnectionAction),
}
impl<T> From<T> for ConnectionMessage
where
	T: Into<ConnectionAction>,
{
	fn from(value: T) -> Self {
		Self::Action(value.into())
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
