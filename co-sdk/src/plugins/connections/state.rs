use co_primitives::{CoId, Did, Network};
use libp2p::{Multiaddr, PeerId};
use std::{
	collections::{BTreeSet, HashMap},
	time::Instant,
};

pub struct ConnectionState {
	pub co: HashMap<CoId, CoConnection>,
	pub networks: HashMap<Network, NetworkConnection>,
	pub cache: HashMap<Network, BTreeSet<Multiaddr>>,
}

#[derive(Debug, Clone)]
pub struct CoConnection {
	pub id: CoId,
	pub from: Did,
	pub networks: BTreeSet<Network>,
	pub keep_alive: Instant,
}

#[derive(Debug, Clone)]
pub struct NetworkConnection {
	pub network: Network,
	pub references: usize,
	pub peers: BTreeSet<PeerId>,
	pub keep_alive: Instant,
}
