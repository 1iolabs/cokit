// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

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
			listen: def.listen.into_iter().map(|s| s.to_string()).collect(),
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
		result.listen = self.listen.parse()?;
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
