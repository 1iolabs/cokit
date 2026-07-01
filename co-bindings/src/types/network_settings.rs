// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use co_sdk::NetworkSettings;
use std::time::Duration;

/// Binding for [`NetworkSettings`].
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
#[derive(Debug, Clone)]
pub struct CoNetworkSettings {
	/// Force to create a new [`PeerId`] on network startup.
	pub force_new_peer_id: bool,

	/// The endpoint to listen to.
	pub listen: String,

	/// The bootstrap peers to increase connectivity.
	pub bootstrap: Vec<String>,

	/// Explicitly configured external addresses.
	/// If the public address of a node is known.
	/// Note: This is required when using relay mode.
	pub external_addresses: Vec<String>,

	/// The default keep alive for connections.
	pub keep_alive_ms: u64,

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
}
impl Default for CoNetworkSettings {
	fn default() -> Self {
		let def = NetworkSettings::default();
		Self {
			force_new_peer_id: def.force_new_peer_id,
			listen: def
				.listen
				.into_iter()
				.map(|addr| addr.to_string())
				.collect::<Vec<_>>()
				.join(","),
			bootstrap: def.bootstrap.into_iter().map(|s| s.to_string()).collect(),
			external_addresses: def.external_addresses.into_iter().map(|s| s.to_string()).collect(),
			keep_alive_ms: def.keep_alive.as_millis().try_into().unwrap_or(u64::MAX),
			peers_threshold: def.peers_threshold,
			relay: def.relay,
			nat: def.nat,
			mdns: def.mdns,
		}
	}
}
impl TryInto<NetworkSettings> for CoNetworkSettings {
	type Error = anyhow::Error;

	fn try_into(self) -> Result<NetworkSettings, anyhow::Error> {
		let mut result = NetworkSettings::default();
		result.force_new_peer_id = self.force_new_peer_id;
		result.listen = self
			.listen
			.split(',')
			.map(|addr| addr.trim())
			.filter(|addr| !addr.is_empty())
			.map(|addr| addr.parse())
			.collect::<Result<_, multiaddr::Error>>()?;
		result.bootstrap = self
			.bootstrap
			.into_iter()
			.map(|addr| addr.parse())
			.collect::<Result<_, multiaddr::Error>>()?;
		result.external_addresses = self
			.external_addresses
			.into_iter()
			.map(|addr| addr.parse())
			.collect::<Result<_, multiaddr::Error>>()?;
		result.keep_alive = Duration::from_millis(self.keep_alive_ms);
		result.peers_threshold = self.peers_threshold;
		result.relay = self.relay;
		result.nat = self.nat;
		result.mdns = self.mdns;
		Ok(result)
	}
}

#[cfg(test)]
mod tests {
	use super::CoNetworkSettings;
	use co_sdk::NetworkSettings;

	#[test]
	fn default_listen_is_comma_joined_dual_stack() {
		let settings = CoNetworkSettings::default();
		assert!(settings.listen.contains("/ip4/0.0.0.0/udp/0/quic-v1"));
		assert!(settings.listen.contains("/ip6/::/udp/0/quic-v1"));
		assert!(settings.listen.contains(','));
	}

	#[test]
	fn comma_separated_listen_parses_into_set() {
		let settings = CoNetworkSettings {
			listen: "/ip4/0.0.0.0/udp/0/quic-v1,/ip6/::/udp/0/quic-v1".to_string(),
			..Default::default()
		};
		let parsed: NetworkSettings = settings.try_into().unwrap();
		assert_eq!(parsed.listen.len(), 2);
	}

	#[test]
	fn comma_separated_with_space_listen_parses_into_set() {
		let settings = CoNetworkSettings {
			listen: "/ip4/0.0.0.0/udp/0/quic-v1, /ip6/::/udp/0/quic-v1".to_string(),
			..Default::default()
		};
		let parsed: NetworkSettings = settings.try_into().unwrap();
		assert_eq!(parsed.listen.len(), 2);
	}
}
