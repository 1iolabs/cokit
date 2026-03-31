// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{
	network::{Behaviour, NetworkEvent},
	services::network::CoNetworkTaskSpawner,
	types::network_task::{NetworkTask, NetworkTaskSpawner},
};
use futures::Stream;
use libp2p::{swarm::SwarmEvent, PeerId, Swarm};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
#[cfg(feature = "js")]
use tokio_with_wasm::alias as tokio;

/// Notify about discovered peers.
#[derive(Debug)]
pub struct PeersNetworkTask {
	tx: mpsc::UnboundedSender<PeerId>,
}
impl PeersNetworkTask {
	pub fn peers(spawner: &CoNetworkTaskSpawner) -> impl Stream<Item = PeerId> + use<> + 'static {
		let (tx, rx) = mpsc::unbounded_channel();
		spawner.spawn(Self { tx }).ok();
		UnboundedReceiverStream::new(rx)
	}
}
impl NetworkTask<Behaviour> for PeersNetworkTask {
	fn execute(&mut self, _swarm: &mut Swarm<Behaviour>) {}

	fn on_swarm_event(
		&mut self,
		_swarm: &mut Swarm<Behaviour>,

		event: SwarmEvent<NetworkEvent>,
	) -> Option<SwarmEvent<NetworkEvent>> {
		#[cfg(feature = "native")]
		if let SwarmEvent::Behaviour(NetworkEvent::Mdns(libp2p::mdns::Event::Discovered(list))) = &event {
			for (peer_id, _) in list {
				self.tx.send(*peer_id).ok();
			}
		}
		Some(event)
	}

	fn is_complete(&mut self) -> bool {
		self.tx.is_closed()
	}
}
