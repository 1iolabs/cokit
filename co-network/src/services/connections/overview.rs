// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use super::state::{ConnectionEndpoint, ConnectionState, PeerConnection};
use crate::services::dns::DnsSource;
use co_primitives::{CoId, Did, Network};
use libp2p::{Multiaddr, PeerId};
use std::collections::BTreeSet;

/// A read-only snapshot of the current network state for diagnostics.
///
/// Combines the local peer id, current listeners, mDNS discoveries, DNS resolver
/// diagnostics and a [`ConnectionSnapshot`] (peers, COs, DIDs, networks, bootstrap).
/// Produced by `NetworkApi::overview` (one-shot) and `NetworkApi::overview_stream` (live).
#[derive(Debug, Clone)]
pub struct NetworkOverview {
	pub local_peer_id: PeerId,
	pub listeners: BTreeSet<Multiaddr>,
	/// Peers currently discovered via mDNS (native only; empty elsewhere).
	pub mdns: BTreeSet<PeerId>,
	pub connections: ConnectionOverview,
	/// DNS resolver diagnostics (`None` on wasm / before first refresh).
	pub dns: Option<DnsSource>,
}

/// How a peer relates to us, derived from its connectivity networks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerRelation {
	/// Direct peer connection (`Network::Peer`).
	Direct,
	/// Discovered via DID discovery.
	DidDiscovery,
	/// Connected via a rendezvous node.
	Rendezvous,
	/// Related via a CO heads topic.
	CoHeads,
}
impl PeerRelation {
	fn from_network(network: &Network) -> Self {
		match network {
			Network::Peer(_) => Self::Direct,
			Network::Rendezvous(_) => Self::Rendezvous,
			Network::DidDiscovery(_) => Self::DidDiscovery,
			Network::CoHeads(_) => Self::CoHeads,
			// `Network` is `#[non_exhaustive]`; treat unknown kinds as the lowest priority.
			_ => Self::CoHeads,
		}
	}

	fn priority(self) -> u8 {
		match self {
			Self::Direct => 0,
			Self::DidDiscovery => 1,
			Self::Rendezvous => 2,
			Self::CoHeads => 3,
		}
	}

	/// The primary relation across a set of networks, by priority (`Direct` wins).
	/// `None` when the peer is related through no known network.
	fn primary(networks: &BTreeSet<Network>) -> Option<Self> {
		networks
			.iter()
			.map(Self::from_network)
			.min_by_key(|relation| relation.priority())
	}
}

/// A connected (or recently related) peer and what it serves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerEntry {
	pub peer_id: PeerId,
	pub connected: bool,
	pub relation: Option<PeerRelation>,
	pub endpoints: Vec<ConnectionEndpoint>,
	pub cos: BTreeSet<CoId>,
	pub dids: BTreeSet<Did>,
	pub networks: BTreeSet<Network>,
}

/// An in-use CO and the networks it connects through.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoEntry {
	pub id: CoId,
	pub from: Did,
	pub networks: BTreeSet<Network>,
}

/// An in-use DID connection and the networks it connects through.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DidEntry {
	pub to: Did,
	pub from: Did,
	pub networks: BTreeSet<Network>,
}

/// A connectivity network and the peers, COs and DIDs referencing it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkEntry {
	pub network: Network,
	pub peers: BTreeSet<PeerId>,
	pub cos: BTreeSet<CoId>,
	pub dids: BTreeSet<Did>,
}

/// A configured bootstrap peer and its dial health.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapEntry {
	pub peer_id: PeerId,
	pub endpoints: BTreeSet<Multiaddr>,
	pub connecting: bool,
	pub failed: u32,
}

/// A read-only, point-in-time snapshot of the connection state.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConnectionOverview {
	pub peers: Vec<PeerEntry>,
	pub cos: Vec<CoEntry>,
	pub dids: Vec<DidEntry>,
	pub networks: Vec<NetworkEntry>,
	pub bootstrap: Vec<BootstrapEntry>,
}

/// DIDs related to a set of networks, via the networks' DID references.
fn dids_for(state: &ConnectionState, networks: &BTreeSet<Network>) -> BTreeSet<Did> {
	networks
		.iter()
		.filter_map(|network| state.networks.get(network))
		.flat_map(|network_connection| network_connection.did_references.iter().cloned())
		.collect()
}

/// Build a [`PeerEntry`] for one tracked peer and what it serves.
fn peer_entry(state: &ConnectionState, peer_id: PeerId, peer_connection: &PeerConnection) -> PeerEntry {
	PeerEntry {
		peer_id,
		connected: peer_connection.connected,
		relation: PeerRelation::primary(&peer_connection.network),
		endpoints: peer_connection.endpoints.values().cloned().collect(),
		cos: peer_connection.co.clone(),
		dids: dids_for(state, &peer_connection.network),
		networks: peer_connection.network.clone(),
	}
}

/// Sort peers connected-first, then by peer id, for a stable layout across refreshes.
fn sort_peers(peers: &mut [PeerEntry]) {
	peers.sort_by(|a, b| b.connected.cmp(&a.connected).then_with(|| a.peer_id.cmp(&b.peer_id)));
}

impl From<&ConnectionState> for ConnectionOverview {
	fn from(state: &ConnectionState) -> Self {
		let mut peers: Vec<PeerEntry> = state
			.peers
			.iter()
			.map(|(peer_id, peer_connection)| peer_entry(state, *peer_id, peer_connection))
			.collect();
		sort_peers(&mut peers);

		let mut cos: Vec<CoEntry> = state
			.co
			.values()
			.map(|co_connection| CoEntry {
				id: co_connection.id.clone(),
				from: co_connection.from.clone(),
				networks: co_connection.networks.clone(),
			})
			.collect();
		cos.sort_by(|a, b| a.id.cmp(&b.id));

		let mut dids: Vec<DidEntry> = state
			.did
			.values()
			.map(|did_connection| DidEntry {
				to: did_connection.to.clone(),
				from: did_connection.from.clone(),
				networks: did_connection.networks.clone(),
			})
			.collect();
		dids.sort_by(|a, b| a.to.cmp(&b.to));

		let mut networks: Vec<NetworkEntry> = state
			.networks
			.values()
			.map(|network_connection| NetworkEntry {
				network: network_connection.network.clone(),
				peers: network_connection.peers.clone(),
				cos: network_connection.references.clone(),
				dids: network_connection.did_references.clone(),
			})
			.collect();
		networks.sort_by(|a, b| a.network.cmp(&b.network));

		let mut bootstrap: Vec<BootstrapEntry> = state
			.bootstrap
			.values()
			.map(|bootstrap_peer| BootstrapEntry {
				peer_id: bootstrap_peer.peer_id,
				endpoints: bootstrap_peer.endpoints.clone(),
				connecting: bootstrap_peer.connecting,
				failed: bootstrap_peer.failed,
			})
			.collect();
		bootstrap.sort_by(|a, b| a.peer_id.cmp(&b.peer_id));

		Self { peers, cos, dids, networks, bootstrap }
	}
}

/// A connection overview scoped to a single CO: the peers serving it and how
/// (per-endpoint transport/direction). Produced by `NetworkApi::co_overview` /
/// `co_overview_stream` for a CO-scoped data-path view (who we're connected to for
/// a chat/group, and over which transport).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoConnectionOverview {
	pub co: CoId,
	/// Peers whose connectivity serves this CO (connected first), with their live endpoints.
	pub peers: Vec<PeerEntry>,
}
impl CoConnectionOverview {
	/// Build the CO-scoped overview directly from the connection state: only the
	/// peers serving `co` (connected first), without materializing the full
	/// [`ConnectionOverview`].
	pub fn from_state(state: &ConnectionState, co: &CoId) -> Self {
		let mut peers: Vec<PeerEntry> = state
			.peers
			.iter()
			.filter(|(_, peer_connection)| peer_connection.co.contains(co))
			.map(|(peer_id, peer_connection)| peer_entry(state, *peer_id, peer_connection))
			.collect();
		sort_peers(&mut peers);
		Self { co: co.clone(), peers }
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::connections::{
		CoConnection, ConnectionAction, ConnectionDirection, ConnectionEndpoint, ConnectionState, NetworkConnection,
		PeerConnection, PeerConnectionClosedAction, PeerConnectionEstablishedAction,
	};
	use co_actor::{time::Instant, Reducer};
	use co_primitives::{NetworkPeer, NetworkRendezvous};
	use libp2p::{swarm::ConnectionId, Multiaddr, PeerId};

	#[test]
	fn projects_faceted_snapshot() {
		let peer = PeerId::random();
		let co: CoId = "co1".into();
		let bob: Did = "did:local:bob".to_string();
		let net_direct = Network::Peer(NetworkPeer { peer: peer.to_bytes(), addresses: vec![] });
		let net_rendezvous = Network::Rendezvous(NetworkRendezvous { namespace: "ns".into(), addresses: vec![] });

		let mut state = ConnectionState::default();
		state.peers.insert(
			peer,
			PeerConnection {
				connected: true,
				co: [co.clone()].into(),
				network: [net_direct.clone(), net_rendezvous.clone()].into(),
				endpoints: [(
					ConnectionId::new_unchecked(1),
					ConnectionEndpoint {
						remote: "/ip4/127.0.0.1/udp/1/quic-v1".parse().unwrap(),
						local: None,
						direction: ConnectionDirection::Outgoing,
						hole_punched: false,
					},
				)]
				.into(),
			},
		);
		state.co.insert(
			co.clone(),
			CoConnection {
				id: co.clone(),
				from: "did:local:me".to_string(),
				networks: [net_direct.clone(), net_rendezvous.clone()].into(),
			},
		);
		state.networks.insert(
			net_rendezvous.clone(),
			NetworkConnection {
				network: net_rendezvous.clone(),
				references: [co.clone()].into(),
				did_references: [bob.clone()].into(),
				peers: [peer].into(),
				keep_alive: Instant::now(),
			},
		);

		let snapshot = ConnectionOverview::from(&state);

		// peer: connected, direct relation (Peer beats Rendezvous), serves co1, carries bob via the rendezvous net.
		assert_eq!(snapshot.peers.len(), 1);
		let peer_entry = &snapshot.peers[0];
		assert_eq!(peer_entry.peer_id, peer);
		assert!(peer_entry.connected);
		assert_eq!(peer_entry.relation, Some(PeerRelation::Direct));
		assert!(peer_entry.cos.contains(&co));
		assert!(peer_entry.dids.contains(&bob));
		assert!(peer_entry
			.endpoints
			.iter()
			.any(|endpoint| endpoint.remote.to_string().contains("quic-v1")));

		// co and network facets are projected too.
		assert_eq!(snapshot.cos.len(), 1);
		assert_eq!(snapshot.cos[0].id, co);
		assert_eq!(snapshot.networks.len(), 1);
		assert!(snapshot.networks[0].dids.contains(&bob));
	}

	#[test]
	fn tracks_peer_endpoints() {
		let mut state = ConnectionState::default();
		let peer = PeerId::random();
		let quic: Multiaddr = "/ip4/127.0.0.1/udp/1/quic-v1".parse().unwrap();
		let tcp: Multiaddr = "/ip4/127.0.0.1/tcp/2".parse().unwrap();

		// two connections to the same peer: one outgoing (QUIC), one incoming (TCP).
		state.reduce(ConnectionAction::PeerConnectionEstablished(PeerConnectionEstablishedAction {
			peer_id: peer,
			connection_id: ConnectionId::new_unchecked(1),
			endpoint: ConnectionEndpoint {
				remote: quic,
				local: None,
				direction: ConnectionDirection::Outgoing,
				hole_punched: false,
			},
			time: Instant::now(),
		}));
		state.reduce(ConnectionAction::PeerConnectionEstablished(PeerConnectionEstablishedAction {
			peer_id: peer,
			connection_id: ConnectionId::new_unchecked(2),
			endpoint: ConnectionEndpoint {
				remote: tcp.clone(),
				local: Some("/ip4/0.0.0.0/tcp/2".parse().unwrap()),
				direction: ConnectionDirection::Incoming,
				hole_punched: false,
			},
			time: Instant::now(),
		}));
		assert_eq!(state.peers[&peer].endpoints.len(), 2);
		assert!(state.peers[&peer]
			.endpoints
			.values()
			.any(|endpoint| endpoint.remote == tcp && endpoint.direction == ConnectionDirection::Incoming));

		// closing one connection drops only its endpoint.
		state.reduce(ConnectionAction::PeerConnectionClosed(PeerConnectionClosedAction {
			peer_id: peer,
			connection_id: ConnectionId::new_unchecked(1),
			time: Instant::now(),
		}));
		assert_eq!(state.peers[&peer].endpoints.len(), 1);
		assert!(state.peers[&peer].endpoints.values().any(|endpoint| endpoint.remote == tcp));
	}
}
