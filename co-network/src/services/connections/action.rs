// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use super::state::ConnectionEndpoint;
use co_actor::time::Instant;
use co_primitives::{CoId, Did, Network};
use derive_more::{From, TryInto};
use libp2p::{swarm::ConnectionId, Multiaddr, PeerId};
use std::collections::BTreeSet;
use uuid::Uuid;

#[derive(Debug, Clone, From, TryInto, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConnectionAction {
	/// Use a CO by utilising the specified networks.
	Use(UseAction),

	/// CO related peers changed.
	PeersChanged(PeersChangedAction),

	/// Release CO.
	/// No active use calls.
	Release(ReleaseAction),

	/// CO has been released.
	Released(ReleasedAction),

	/// Resolve CO networks.
	NetworkResolve(NetworkResolveAction),

	/// CO networks has been resolved.
	NetworkResolveComplete(NetworkResolveCompleteAction),

	/// Connect to a network.
	///
	/// Possible Responses:
	/// - [`ConnectionAction::Connected`]
	/// - [`ConnectionAction::Disconnected`]
	Connect(ConnectAction),

	/// Network has been connected.
	/// May be executed multiple times when connections to a network change.
	Connected(ConnectedAction),

	/// Disconnect network (entirely).
	Disconnect(DisconnectAction),

	/// Relate a PeerId to a Co.
	/// This will make the peer to be returned when a Co connection is requested.
	///
	/// Security: This relation must be known to be true by the caller.
	PeerRelateCo(PeerRelateCoAction),

	/// Relate a PeerId to a DID.
	///
	/// Security: This relation must be known to be true (trusted) by the caller.
	PeerRelateDid(PeerRelateDidAction),

	/// Network has been (entirely) disconnected.
	Disconnected(DisconnectedAction),

	/// A connection to a peer has been established.
	/// Fired for **every** connection.
	PeerConnectionEstablished(PeerConnectionEstablishedAction),

	/// A connection to a peer has been closed.
	/// Fired for **every** connection.
	PeerConnectionClosed(PeerConnectionClosedAction),

	/// A relayed connection to a peer was upgraded to a direct one via hole-punching (libp2p-dcutr).
	PeerHolePunched(PeerHolePunchedAction),

	/// Try to dial a peer.
	Dial(DialAction),

	/// Dial a peer has been completed.
	DialCompleted(DialCompletedAction),

	/// Use a DID connection by utilising the specified networks.
	///
	/// Raw message callers must retain [`DidUseAction::release`], close or drop the
	/// response receiver, and then dispatch that exact release action.
	DidUse(DidUseAction),

	/// DID related peers changed.
	DidPeersChanged(DidPeersChangedAction),

	/// Release DID connection.
	DidRelease(DidReleaseAction),

	/// DID connection has been released.
	DidReleased(DidReleasedAction),

	/// Notify about insufficient peers.
	/// That causes to increase connectivity by dialing bootstrap peers.
	InsufficentPeers,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct UseAction {
	pub id: CoId,
	pub from: Did,
	pub time: Instant,

	/// The networks to use.
	/// If empty the networks will be resolved using the CO settings.
	///
	/// # Guaranties
	/// - Network resolve will not use networking to prevent loops.
	/// - If at least one network is passed no automatic resolve will happen.
	pub networks: BTreeSet<Network>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PeersChangedAction {
	pub id: CoId,
	pub peers: BTreeSet<PeerId>,
	pub added: BTreeSet<PeerId>,
	pub removed: BTreeSet<PeerId>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReleaseAction {
	pub id: CoId,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReleasedAction {
	pub id: CoId,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NetworkResolveAction {
	pub id: CoId,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NetworkResolveCompleteAction {
	pub id: CoId,
	pub result: Result<BTreeSet<Network>, String>,
	pub time: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConnectAction {
	pub network: Network,
	pub from: Did,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConnectedAction {
	pub network: Network,
	pub result: Result<BTreeSet<PeerId>, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DisconnectAction {
	pub network: Network,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DisconnectedAction {
	pub network: Network,
	pub reason: DisconnectReason,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PeerConnectionEstablishedAction {
	pub peer_id: PeerId,
	pub connection_id: ConnectionId,
	pub endpoint: ConnectionEndpoint,
	pub time: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PeerConnectionClosedAction {
	pub peer_id: PeerId,
	pub connection_id: ConnectionId,
	pub time: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PeerHolePunchedAction {
	pub peer_id: PeerId,
	/// The direct connection the hole-punch successfully created.
	pub connection_id: ConnectionId,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, thiserror::Error)]
pub enum DisconnectReason {
	#[error("No network available to connect")]
	NoNetwork,
	#[error("Failure before connect")]
	Failure(String),
	#[error("Connect Timeout")]
	Timeout,
	#[error("Close")]
	Close,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PeerRelateCoAction {
	pub peer_id: PeerId,
	pub co: CoId,
	pub did: Option<Did>,
	pub time: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PeerRelateDidAction {
	pub peer_id: PeerId,
	pub did: Did,
	pub time: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DialAction {
	pub peer_id: PeerId,
	pub endpoints: BTreeSet<Multiaddr>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DialCompletedAction {
	pub peer_id: PeerId,
	pub ok: bool,
	pub time: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DidUseAction {
	/// Unique lease paired with the matching [`DidReleaseAction`].
	pub lease_id: DidUseLeaseId,
	pub from: Did,
	pub to: Did,
	pub time: Instant,
	pub networks: BTreeSet<Network>,
}
impl DidUseAction {
	pub fn new(from: Did, to: Did, time: Instant, networks: BTreeSet<Network>) -> Self {
		Self { lease_id: DidUseLeaseId::new(), from, to, time, networks }
	}

	/// Create the release action paired with this use.
	pub fn release(&self) -> DidReleaseAction {
		DidReleaseAction::new(self.to.clone(), self.lease_id)
	}
}

/// Unique identity of one DID connection use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DidUseLeaseId(Uuid);
impl DidUseLeaseId {
	pub fn new() -> Self {
		Self(Uuid::new_v4())
	}
}
impl Default for DidUseLeaseId {
	fn default() -> Self {
		Self::new()
	}
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DidPeersChangedAction {
	pub to: Did,
	pub peers: BTreeSet<PeerId>,
	pub added: BTreeSet<PeerId>,
	pub removed: BTreeSet<PeerId>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DidReleaseAction {
	/// Lease created by the matching [`DidUseAction`].
	pub lease_id: DidUseLeaseId,
	pub to: Did,
}
impl DidReleaseAction {
	pub fn new(to: Did, lease_id: DidUseLeaseId) -> Self {
		Self { lease_id, to }
	}
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DidReleasedAction {
	pub to: Did,
}
