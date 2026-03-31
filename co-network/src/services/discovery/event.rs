// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use libp2p::PeerId;

/// Event.
#[derive(Debug, Clone)]
pub enum Event {
	// Resolved { id: u64, request: Discovery, peers: BTreeMap<PeerId, Vec<Multiaddr>> },
	/// A peer to be discovered has connected.
	Connected { id: u64, peer: PeerId },

	/// A peer to be discovered has disconnected.
	Disconnected { id: u64, peer: PeerId },

	/// A discovery connect has insufficient peers to make a connection (and asks for more connections).
	InsufficentPeers { id: u64 },

	/// All discovery attempts have been exhausted with no connections.
	Failed { id: u64 },

	/// A discovery connect has timed-out.
	Timeout { id: u64 },
}
