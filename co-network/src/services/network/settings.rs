// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::try_peer_id;
use multiaddr::Multiaddr;
use std::{collections::BTreeSet, time::Duration};

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct NetworkSettings {
	/// Force to create a new [`PeerId`] on network startup.
	pub force_new_peer_id: bool,

	/// The endpoints to listen to.
	pub listen: BTreeSet<Multiaddr>,

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

	/// Whether to enable a limited relay server.
	/// This relay can be used by other peers for hole-punching.
	pub relay: bool,

	/// Enable NAT related protocols.
	pub nat: bool,

	/// Enable mDNS protocol.
	pub mdns: bool,

	/// Enable Websocket protocol.
	///
	/// # Notes
	/// - Required to let browsers connect to this instance.
	/// - Not available on mobile.
	pub websocket: bool,

	/// DNS configuration.
	pub dns: NetworkDns,

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
			websocket: true,
			dns: Default::default(),
			max_circuit_bytes: None,
			max_circuit_duration: None,
		}
	}
}
impl NetworkSettings {
	pub fn new() -> Self {
		Self::default()
	}

	/// Mobile configuration.
	pub fn mobile() -> Self {
		Self { websocket: false, ..Default::default() }
	}

	/// Web configuration.
	///
	/// Currently a webrtc relay is required to connect via. web browser.
	/// Example: `/ip4/127.0.0.1/tcp/4001/ws/p2p/12D3KooWGW7HBqhnY9wN9F9JWc72SYrb15nKWyGfLZDDYR8KA17x`
	/// Note that the p2p part is currently required.
	#[cfg(feature = "web")]
	pub fn web(relay_multiaddr: &str) -> Result<Self, anyhow::Error> {
		Self { mdns: false, nat: true, relay: false, bootstrap: Default::default(), ..Default::default() }
			.with_bootstrap_from_string(relay_multiaddr)
	}

	fn default_listen() -> BTreeSet<Multiaddr> {
		["/ip4/0.0.0.0/udp/0/quic-v1", "/ip6/::/udp/0/quic-v1"]
			.into_iter()
			.map(|addr| addr.parse().expect("to parse"))
			.collect()
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

	/// Set listen endpoint, replacing any existing listen addresses.
	pub fn with_listen(mut self, listen: Multiaddr) -> Self {
		self.listen = [listen].into_iter().collect();
		self
	}

	/// Set listen endpoints, replacing any existing listen addresses.
	pub fn with_listens(mut self, listen: impl IntoIterator<Item = Multiaddr>) -> Self {
		self.listen = listen.into_iter().collect();
		self
	}

	/// Add a listen endpoint.
	pub fn with_added_listen(mut self, listen: Multiaddr) -> Self {
		self.listen.insert(listen);
		self
	}

	/// Set listen endpoint from a string, replacing any existing listen addresses.
	pub fn with_listen_from_string(mut self, listen: &str) -> Result<Self, anyhow::Error> {
		self.listen = [listen.parse()?].into_iter().collect();
		Ok(self)
	}

	/// Local-only profile for tests/dev: a loopback TCP listener with mDNS, NAT and bootstrap
	/// disabled.
	///
	/// None of them are meaningful on `127.0.0.1`, and same-machine mDNS actively
	/// interferes — it advertises the host's LAN IP, where loopback peers do not listen, so peers
	/// get stuck dialing the wrong address.
	///
	/// Connect localhost peers with an explicit `dial`.
	pub fn with_localhost(self) -> Self {
		self.with_listen("/ip4/127.0.0.1/tcp/0".parse().unwrap())
			.without_bootstrap()
			.with_nat(false)
			.with_mdns(false)
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

#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub enum NetworkDns {
	/// Do not use any DNS.
	None,

	/// Use system configuration, with automatic static fallback and live refresh.
	///
	/// # Note
	/// - Linux + macOS: `/etc/resolv.conf` mtime drives per-dial staleness detection
	/// - `recover()` force-refreshes on all native platforms
	/// - Note: iOS/Windows/Android have no usable staleness file and rely on `recover()` alone
	/// - Falls back to the preconfigured (Cloudflare) resolver while the OS has no nameservers
	#[default]
	System,

	/// Use preconfigured Cloudflare DNS.
	Cloudflare,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_listen_is_dual_stack_quic() {
		let settings = NetworkSettings::default();
		assert!(settings.listen.contains(&"/ip4/0.0.0.0/udp/0/quic-v1".parse().unwrap()));
		assert!(settings.listen.contains(&"/ip6/::/udp/0/quic-v1".parse().unwrap()));
		assert_eq!(settings.listen.len(), 2);
	}

	#[test]
	fn with_listen_replaces_with_single() {
		let settings = NetworkSettings::default().with_listen("/ip4/127.0.0.1/tcp/0".parse().unwrap());
		assert_eq!(settings.listen.len(), 1);
		assert!(settings.listen.contains(&"/ip4/127.0.0.1/tcp/0".parse().unwrap()));
	}

	#[test]
	fn with_added_listen_inserts() {
		let extra: Multiaddr = "/ip6/::/udp/4001/quic-v1".parse().unwrap();
		let settings = NetworkSettings::default().with_added_listen(extra.clone());
		assert!(settings.listen.contains(&extra));
		assert_eq!(settings.listen.len(), 3);
	}

	#[test]
	fn mobile_uses_system_dns() {
		let settings = NetworkSettings::mobile();
		assert!(matches!(settings.dns, NetworkDns::System));
		assert!(!settings.websocket);
	}
}
