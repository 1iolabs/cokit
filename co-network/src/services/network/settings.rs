// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::try_peer_id;
use multiaddr::Multiaddr;
use std::{collections::BTreeSet, time::Duration};

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct NetworkSettings {
	/// Force to create a new [`PeerId`] on network startup.
	pub force_new_peer_id: bool,

	/// The endpoint to listen to.
	pub listen: Multiaddr,

	/// The bootstrap peers to increase connectivity.
	pub bootstrap: BTreeSet<Multiaddr>,

	/// Explicitly configured external addresses.
	/// If the public address of a node is known.
	/// Note: This is required when using relay mode.
	pub external_addresses: Vec<Multiaddr>,

	/// The default keep alive for connections.
	pub keep_alive: Duration,

	/// Number of peers to keep connected.
	/// More peers will be discoverd using bootstrap when the count falls below this number.
	/// This is optional and if it is set to [`None`] all connections are only on demand.
	pub peers_threshold: Option<u32>,

	/// Wherther to enable a limited relay server.
	/// This relay can be used by other peers for holepunching.
	pub relay: bool,

	/// Enable NAT related protocols.
	pub nat: bool,

	/// Enable mDNS protocol.
	pub mdns: bool,

	/// Maximum number of bytes allowed on a relay circuit.
	/// If `None`, the libp2p default (128 KiB) is used.
	pub max_circuit_bytes: Option<u64>,

	/// Maximum duration of a relay circuit.
	/// If `None`, the libp2p default (120s) is used.
	pub max_circuit_duration: Option<Duration>,
}
impl Default for NetworkSettings {
	fn default() -> Self {
		Self {
			force_new_peer_id: Default::default(),
			listen: Self::default_listen(),
			bootstrap: Self::default_bootstrap(),
			external_addresses: Default::default(),
			keep_alive: Duration::from_secs(30),
			peers_threshold: Some(10),
			relay: false,
			nat: true,
			mdns: true,
			max_circuit_bytes: None,
			max_circuit_duration: None,
		}
	}
}
impl NetworkSettings {
	pub fn new() -> Self {
		Self::default()
	}

	#[cfg(feature = "web")]
	pub fn web() -> Self {
		Self { mdns: false, nat: true, relay: false, ..Default::default() }
	}

	fn default_listen() -> Multiaddr {
		"/ip4/0.0.0.0/udp/0/quic-v1".parse().expect("to parse")
	}

	fn default_bootstrap() -> BTreeSet<Multiaddr> {
		let bootstrap =
			["/dns4/bootstrap.1io.com/udp/5000/quic-v1/p2p/12D3KooWEinh2zCgGbJaDfepoiiPiBgFcysSMYSc1EQrgEEZi9aX"];
		bootstrap.into_iter().map(|s| s.parse().expect("to parse")).collect()
	}

	pub fn with_force_new_peer_id(mut self, value: bool) -> Self {
		self.force_new_peer_id = value;
		self
	}

	/// Set listen endpoint.
	pub fn with_listen(mut self, listen: Multiaddr) -> Self {
		self.listen = listen;
		self
	}

	/// Set listen endpoint.
	pub fn with_listen_from_string(mut self, listen: &str) -> Result<Self, anyhow::Error> {
		self.listen = listen.parse()?;
		Ok(self)
	}

	/// Set local listen endpoint.
	pub fn with_localhost(mut self) -> Self {
		self.listen = "/ip4/127.0.0.1/tcp/0".parse().unwrap();
		self
	}

	/// Clear all bootstrap endpoints.
	pub fn without_bootstrap(mut self) -> Self {
		self.bootstrap.clear();
		self
	}

	/// Set bootstrap endpoint.
	pub fn with_bootstrap(mut self, bootstrap: Multiaddr) -> Self {
		self.bootstrap = [bootstrap].into_iter().collect();
		self
	}

	/// Set bootstrap endpoint.
	pub fn with_bootstraps(mut self, bootstrap: impl IntoIterator<Item = Multiaddr>) -> Self {
		self.bootstrap = bootstrap.into_iter().collect();
		self
	}

	/// Add bootstrap endpoint.
	pub fn with_added_bootstrap(mut self, bootstrap: Multiaddr) -> Self {
		self.bootstrap.insert(bootstrap);
		self
	}

	/// Add bootstrap endpoint.
	pub fn with_added_bootstraps(mut self, bootstrap: impl IntoIterator<Item = Multiaddr>) -> Self {
		self.bootstrap.extend(bootstrap);
		self
	}

	/// Add bootstrap endpoint.
	pub fn with_bootstrap_from_string(mut self, bootstrap: &str) -> Result<Self, anyhow::Error> {
		self.bootstrap.insert(bootstrap.parse()?);
		Ok(self)
	}

	/// Add external address.
	pub fn with_added_external_address(mut self, external_address: Multiaddr) -> Self {
		self.external_addresses.push(external_address);
		self
	}

	/// Add external addresses.
	pub fn with_added_external_addresses(mut self, external_address: impl IntoIterator<Item = Multiaddr>) -> Self {
		self.external_addresses.extend(external_address);
		self
	}

	/// Enable relay mode to allow hole-punching over this swarm.
	pub fn with_relay(mut self, relay: bool) -> Self {
		self.relay = relay;
		self
	}

	/// Enable mDNS protocol.
	pub fn with_mdns(mut self, mdns: bool) -> Self {
		self.mdns = mdns;
		self
	}

	/// Enable NAT related protocols.
	pub fn with_nat(mut self, nat: bool) -> Self {
		self.nat = nat;
		self
	}

	/// Set the maximum number of bytes allowed on a relay circuit.
	pub fn with_max_circuit_bytes(mut self, max_circuit_bytes: u64) -> Self {
		self.max_circuit_bytes = Some(max_circuit_bytes);
		self
	}

	/// Set the maximum duration of a relay circuit.
	pub fn with_max_circuit_duration(mut self, max_circuit_duration: Duration) -> Self {
		self.max_circuit_duration = Some(max_circuit_duration);
		self
	}

	/// Validate if settings are correct.
	pub fn build(self) -> Result<Self, anyhow::Error> {
		for bootstrap in self.bootstrap.iter() {
			try_peer_id(bootstrap)?;
		}
		Ok(self)
	}
}
