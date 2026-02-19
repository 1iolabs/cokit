// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use co_primitives::{CoId, Did, Network};
use derive_more::{From, TryInto};
use libp2p::{Multiaddr, PeerId};
use std::{collections::BTreeSet, time::Instant};

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
	NetworkResolved(NetworkResolvedAction),

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
	PeerConnectionEstablished(PeerConnectionEstablishedAction),

	/// A connection to a peer has been closed.
	PeerConnectionClosed(PeerConnectionClosedAction),

	/// Try to dial a peer.
	Dial(DialAction),

	/// Dial a peer has been completed.
	DialCompleted(DialCompletedAction),

	/// Notify about insufficent peers.
	/// That causes to increase connectivity by dailing bootstrap peers.
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
pub struct NetworkResolvedAction {
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
	pub time: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PeerConnectionClosedAction {
	pub peer_id: PeerId,
	pub time: Instant,
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
