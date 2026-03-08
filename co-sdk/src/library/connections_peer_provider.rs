// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use co_actor::{time, ActorHandle};
use co_network::{
	connections::{ConnectionMessage, UseAction},
	PeerId, PeerProvider,
};
use co_primitives::{CoId, Did};
use futures::{Stream, TryStreamExt};
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct ConnectionsPeerProvider {
	id: CoId,
	from: Did,
	connections: ActorHandle<ConnectionMessage>,
}
impl ConnectionsPeerProvider {
	pub fn new(id: CoId, from: Did, connections: ActorHandle<ConnectionMessage>) -> Self {
		Self { id, from, connections }
	}
}
impl PeerProvider for ConnectionsPeerProvider {
	fn try_peers(&self) -> impl Stream<Item = Result<BTreeSet<PeerId>, anyhow::Error>> + Send + 'static {
		let action = UseAction {
			id: self.id.clone(),
			from: self.from.clone(),
			time: time::Instant::now(),
			networks: Default::default(),
		};
		self.connections
			.stream(|response| ConnectionMessage::Use(action, response))
			.map_ok(|change| change.peers)
			.map_err(Into::into)
	}
}
