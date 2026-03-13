use libp2p::PeerId;

/// Event.
#[derive(Debug, Clone)]
pub enum Event {
	// Resolved { id: u64, request: Discovery, peers: BTreeMap<PeerId, Vec<Multiaddr>> },
	/// A peer to be discovered has connected.
	Connected { id: u64, peer: PeerId },

	/// A peer to be discovered has disconnected.
	Disconnected { id: u64, peer: PeerId },

	/// A discovery connect has infufficent peers to amke a connection.
	InsufficentPeers { id: u64 },

	/// A discovery connect has timedout.
	/// TODO: Does it always mean it has failed?
	Timeout { id: u64 },
}
