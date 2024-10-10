use co_primitives::{CoId, Did, Network};
use derive_more::{From, TryInto};
use libp2p::PeerId;
use std::{collections::BTreeSet, time::Instant};

#[derive(Debug, Clone, From, TryInto, PartialEq)]
pub enum ConnectionAction {
	/// Use a CO by utilitsing the specified networks.
	Use(UseAction),

	/// CO related peers changed.
	PeersChanged(PeersChangedAction),

	/// Release CO.
	Release(ReleaseAction),

	/// CO has been released.
	Released(ReleasedAction),

	/// Connect to a network.
	Connect(ConnectAction),

	/// Network has been connected.
	/// May be executed multiple times when connections to a network change.
	Connected(ConnectedAction),

	/// Network has been (entirely) disconnected.
	Disconnected(DisconnectedAction),
}

#[derive(Debug, Clone, PartialEq)]
pub struct UseAction {
	pub id: CoId,
	pub from: Did,
	pub time: Instant,
	pub networks: BTreeSet<Network>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PeersChangedAction {
	pub id: CoId,
	pub peers: BTreeSet<PeerId>,
	pub added: BTreeSet<PeerId>,
	pub removed: BTreeSet<PeerId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReleaseAction {
	pub id: CoId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReleasedAction {
	pub id: CoId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectAction {
	pub network: Network,
	pub from: Did,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectedAction {
	pub network: Network,
	pub result: Result<BTreeSet<PeerId>, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DisconnectedAction {
	pub network: Network,
	pub reason: DisconnectReason,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum DisconnectReason {
	#[error("No network available to connect")]
	NoNetwork,
	#[error("Failure before connect")]
	Failure(String),
	#[error("Connect Timeout")]
	Timeout,
}
