// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

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
