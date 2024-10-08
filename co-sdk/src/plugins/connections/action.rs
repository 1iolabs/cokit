use co_primitives::{CoId, Did, Network};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, time::Instant};

pub enum ConnectionAction {
	/// Use a CO by utilitsing the specified networks.
	Use(CoId, Did, Instant, BTreeSet<Network>),

	/// Connect to a network.
	Connect(NetworkWithContext),

	/// Network has been connected.
	/// May be executed multiple times when connections to a network change.
	Connected(Network, Result<BTreeSet<PeerId>, anyhow::Error>),

	/// Network has been (entirely) disconnected.
	Disconnected(Network, DisconnectReason),
	// AddExplictPeer(CoId, PeerId),
	// RemoveExplictPeer(CoId, PeerId),
}

#[derive(Debug, thiserror::Error)]
pub enum DisconnectReason {
	#[error("No network available to connect")]
	NoNetwork,
	#[error("Failure before connect")]
	Failure(#[source] anyhow::Error),
	#[error("Connect Timeout")]
	Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct NetworkWithContext {
	pub from: Did,
	pub network: Network,
}
