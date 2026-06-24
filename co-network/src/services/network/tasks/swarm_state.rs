// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{
	network::{Behaviour, NetworkEvent},
	services::network::CoNetworkTaskSpawner,
	types::network_task::{NetworkTask, NetworkTaskSpawner},
};
use futures::Stream;
use libp2p::{swarm::SwarmEvent, Multiaddr, PeerId, Swarm};
use std::collections::BTreeSet;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
#[cfg(feature = "js")]
use tokio_with_wasm::alias as tokio;

/// The swarm-side overview state: current listeners + mDNS-discovered peers.
pub type SwarmState = (BTreeSet<Multiaddr>, BTreeSet<PeerId>);

/// Stream the swarm-side overview state (listeners + mDNS discoveries), read
/// directly from the swarm and re-emitted whenever a relevant swarm event changes
/// it.
#[derive(Debug)]
pub struct SwarmStateWatchTask {
	tx: mpsc::UnboundedSender<SwarmState>,
}
impl SwarmStateWatchTask {
	pub fn watch(spawner: &CoNetworkTaskSpawner) -> impl Stream<Item = SwarmState> + use<> + 'static {
		let (tx, rx) = mpsc::unbounded_channel();
		spawner.spawn(Self { tx }).ok();
		UnboundedReceiverStream::new(rx)
	}

	/// Read listeners (local + external) and mDNS discoveries directly from the swarm.
	fn read(swarm: &Swarm<Behaviour>) -> SwarmState {
		let mut listeners: BTreeSet<Multiaddr> = BTreeSet::new();
		listeners.extend(swarm.listeners().cloned());
		listeners.extend(swarm.external_addresses().cloned());
		#[cfg(feature = "native")]
		let mdns = swarm
			.behaviour()
			.mdns
			.as_ref()
			.map(|mdns| mdns.discovered_nodes().copied().collect::<BTreeSet<PeerId>>())
			.unwrap_or_default();
		#[cfg(not(feature = "native"))]
		let mdns = BTreeSet::<PeerId>::new();
		(listeners, mdns)
	}
}
impl NetworkTask<Behaviour> for SwarmStateWatchTask {
	fn execute(&mut self, swarm: &mut Swarm<Behaviour>) {
		// initial read so the subscriber gets the current state immediately.
		self.tx.send(Self::read(swarm)).ok();
	}

	fn on_swarm_event(
		&mut self,
		swarm: &mut Swarm<Behaviour>,
		event: SwarmEvent<NetworkEvent>,
	) -> Option<SwarmEvent<NetworkEvent>> {
		let listener_changed = matches!(
			&event,
			SwarmEvent::NewListenAddr { .. }
				| SwarmEvent::ExpiredListenAddr { .. }
				| SwarmEvent::ListenerClosed { .. }
				| SwarmEvent::ExternalAddrConfirmed { .. }
		);
		#[cfg(feature = "native")]
		let changed = listener_changed || matches!(&event, SwarmEvent::Behaviour(NetworkEvent::Mdns(_)));
		#[cfg(not(feature = "native"))]
		let changed = listener_changed;
		if changed {
			self.tx.send(Self::read(swarm)).ok();
		}
		Some(event)
	}

	fn is_complete(&mut self) -> bool {
		self.tx.is_closed()
	}
}
