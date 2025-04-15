use crate::services::connections::{ConnectionMessage, UseAction};
use co_actor::ActorHandle;
use co_network::PeerProvider;
use co_primitives::{CoId, Did};
use futures::{Stream, TryStreamExt};
use libp2p::PeerId;
use std::{collections::BTreeSet, time::Instant};

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
			time: Instant::now(),
			networks: Default::default(),
		};
		self.connections
			.stream(|response| ConnectionMessage::Use(action, response))
			.map_ok(|change| change.peers)
			.map_err(Into::into)
	}
}
