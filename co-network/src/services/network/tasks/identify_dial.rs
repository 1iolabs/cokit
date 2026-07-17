// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{
	network::{Behaviour, NetworkEvent},
	services::network::{known_peer_dial_opts, DialIntent},
	types::network_task::NetworkTask,
};
use ipnet::IpNet;
use libp2p::{
	identify,
	swarm::{ConnectionId, SwarmEvent},
	Multiaddr, PeerId, Swarm,
};
use multiaddr::Protocol;
use std::{
	collections::{HashMap, HashSet},
	net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

/// Dial all listen addresses when identifies a peer.
/// This supports establishing bidirectional connectivity.
#[derive(Debug)]
pub struct IdentifyDialNetworkTask {
	agent: String,
	/// Addresses already dialed per peer, so repeated identify pushes do not
	/// open duplicate connections. Released again when their dial fails, so
	/// transient failures (e.g. a pending local-network permission prompt) are
	/// retried on the next identify. Cleared when a peer fully disconnects.
	dialed: HashMap<PeerId, HashSet<Multiaddr>>,
	/// Upgrade dials in flight, keyed by their dial connection id, to resolve
	/// each dial's outcome back to the addresses it attempted.
	pending: HashMap<ConnectionId, (PeerId, Vec<Multiaddr>)>,
}
impl IdentifyDialNetworkTask {
	pub fn new(agent: String) -> Self {
		Self { agent, dialed: HashMap::new(), pending: HashMap::new() }
	}

	/// Dialable addresses for `peer_id` not already attempted
	fn addresses_to_dial(
		&mut self,
		peer_id: PeerId,
		listen_addrs: &[Multiaddr],
		local_nets: &[IpNet],
	) -> Vec<Multiaddr> {
		let attempted = self.dialed.entry(peer_id).or_default();
		let mut to_dial = Vec::new();
		for addr in listen_addrs {
			if is_dialable_addr(addr, local_nets) && attempted.insert(addr.clone()) {
				to_dial.push(addr.clone());
			}
		}
		to_dial
	}

	/// Forget a fully-disconnected peer so a future reconnection can re-dial.
	fn forget_peer(&mut self, peer_id: &PeerId) {
		self.dialed.remove(peer_id);
	}

	/// Remember an in-flight upgrade dial to track its outcome.
	fn record_pending(&mut self, connection_id: ConnectionId, peer_id: PeerId, addresses: Vec<Multiaddr>) {
		self.pending.insert(connection_id, (peer_id, addresses));
	}

	/// A pending upgrade dial resolved into a connection; keep its addresses
	/// marked so repeated identifies do not open duplicate connections.
	fn dial_established(&mut self, connection_id: &ConnectionId) -> bool {
		self.pending.remove(connection_id).is_some()
	}

	/// A pending upgrade dial failed entirely; release its addresses so the
	/// next identify of the peer may retry them.
	fn dial_failed(&mut self, connection_id: &ConnectionId) -> Option<PeerId> {
		let (peer_id, addresses) = self.pending.remove(connection_id)?;
		self.unmark(&peer_id, &addresses);
		Some(peer_id)
	}

	/// Release `addresses` from the attempted set of `peer_id`.
	fn unmark(&mut self, peer_id: &PeerId, addresses: &[Multiaddr]) {
		if let Some(attempted) = self.dialed.get_mut(peer_id) {
			for address in addresses {
				attempted.remove(address);
			}
			if attempted.is_empty() {
				self.dialed.remove(peer_id);
			}
		}
	}
}
impl NetworkTask<Behaviour> for IdentifyDialNetworkTask {
	fn execute(&mut self, _swarm: &mut Swarm<Behaviour>) {}

	fn on_swarm_event(
		&mut self,
		swarm: &mut Swarm<Behaviour>,

		event: SwarmEvent<NetworkEvent>,
	) -> Option<SwarmEvent<NetworkEvent>> {
		match &event {
			SwarmEvent::Behaviour(NetworkEvent::Identify(identify::Event::Received { peer_id, info, .. })) => {
				if info.agent_version == self.agent {
					let peer_id = *peer_id;
					let local_nets = local_subnets();
					let to_dial = self.addresses_to_dial(peer_id, &info.listen_addrs, &local_nets);
					if !to_dial.is_empty() {
						let opts = known_peer_dial_opts(peer_id, to_dial.clone(), DialIntent::Redundancy);
						let connection_id = opts.connection_id();
						match swarm.dial(opts) {
							Ok(_) => {
								self.record_pending(connection_id, peer_id, to_dial.clone());
								tracing::debug!(?peer_id, ?to_dial, "network-identify-dial");
							},
							Err(err) => {
								// rejected synchronously (e.g. a concurrent pending dial):
								// release the addresses so the next identify retries them.
								self.unmark(&peer_id, &to_dial);
								tracing::debug!(?err, ?peer_id, ?to_dial, "network-identify-dial-rejected");
							},
						}
					}
				}
			},
			SwarmEvent::ConnectionEstablished { connection_id, endpoint, .. } => {
				if self.dial_established(connection_id) {
					tracing::debug!(remote = ?endpoint.get_remote_address(), "network-identify-dial-established");
				}
			},
			SwarmEvent::OutgoingConnectionError { connection_id, error, .. } => {
				if let Some(peer_id) = self.dial_failed(connection_id) {
					tracing::info!(?peer_id, ?error, "network-identify-dial-error");
				}
			},
			SwarmEvent::ConnectionClosed { peer_id, num_established, .. } => {
				// forget peers fully disconnected
				if *num_established == 0 {
					self.forget_peer(peer_id);
				}
			},
			_ => {},
		}
		Some(event)
	}

	fn is_complete(&mut self) -> bool {
		false
	}
}

/// Whether `addr` is worth a direct dial: a reachable, non-proximity transport
/// address. Public IPs are always dialable; private IPs only when they fall
/// inside one of our own local subnets (`local_nets`). BLE and relay-circuit
/// addresses are never dialable - we never "upgrade" to those.
pub fn is_dialable_addr(addr: &Multiaddr, local_nets: &[IpNet]) -> bool {
	// never upgrade to a relay circuit.
	for proto in addr.iter() {
		if let Protocol::P2pCircuit = proto {
			return false;
		}
	}

	// decide on the IP component, if any.
	for proto in addr.iter() {
		match proto {
			Protocol::Ip4(ip) => return is_dialable_ipv4(ip, local_nets),
			Protocol::Ip6(ip) => return is_dialable_ipv6(ip, local_nets),
			_ => continue,
		}
	}

	// no IP component (e.g. /dns*) - let dialing resolve it.
	true
}

fn is_dialable_ipv4(ip: Ipv4Addr, local_nets: &[IpNet]) -> bool {
	if ip.is_loopback() || ip.is_link_local() || ip.is_unspecified() || ip.is_broadcast() || ip.is_multicast() {
		return false;
	}

	// 192.0.0.0/24 (incl. the 464xlat clat stub) is not globally routable.
	if is_ietf_protocol_ipv4(ip) {
		return false;
	}

	// RFC1918 private or RFC6598 CGNAT (100.64.0.0/10): not globally routable -
	// dial only if it is one of our own local networks.
	if ip.is_private() || is_cgnat_ipv4(ip) {
		return local_nets.iter().any(|net| net.contains(&IpAddr::V4(ip)));
	}

	// globally routable.
	true
}

/// RFC 6598 shared address space (100.64.0.0/10), used for carrier-grade NAT.
fn is_cgnat_ipv4(ip: Ipv4Addr) -> bool {
	u32::from(ip) & 0xffc0_0000 == 0x6440_0000
}

/// rfc 6890 / rfc 7335 ietf protocol assignments (192.0.0.0/24), which contains the 464xlat clat
/// stub (192.0.0.0/29); not globally routable.
fn is_ietf_protocol_ipv4(ip: Ipv4Addr) -> bool {
	u32::from(ip) & 0xffff_ff00 == 0xc000_0000
}

fn is_dialable_ipv6(ip: Ipv6Addr, local_nets: &[IpNet]) -> bool {
	if ip.is_loopback() || ip.is_unspecified() || ip.is_multicast() {
		return false;
	}
	let seg0 = ip.segments()[0];

	// link-local (fe80::/10): needs a zone id we cannot carry - skip.
	if seg0 & 0xffc0 == 0xfe80 {
		return false;
	}

	// unique Local Address (fc00::/7): dial only if in one of our subnets.
	if seg0 & 0xfe00 == 0xfc00 {
		return local_nets.iter().any(|net| net.contains(&IpAddr::V6(ip)));
	}

	// global.
	true
}

/// Snapshot of our own interface subnets (CIDRs), used to decide whether a
/// peer's private listen address is on one of our local networks. Empty on
/// wasm - browsers have no dialable local interfaces.
#[cfg(not(target_arch = "wasm32"))]
fn local_subnets() -> Vec<IpNet> {
	let ifaces = match if_addrs::get_if_addrs() {
		Ok(ifaces) => ifaces,
		Err(err) => {
			tracing::debug!(?err, "network-identify-dial-ifaddrs-failed");
			return Vec::new();
		},
	};
	ifaces
		.into_iter()
		.filter(|iface| !iface.is_loopback())
		.filter_map(|iface| {
			let (ip, netmask) = match iface.addr {
				if_addrs::IfAddr::V4(v4) => (IpAddr::V4(v4.ip), IpAddr::V4(v4.netmask)),
				if_addrs::IfAddr::V6(v6) => (IpAddr::V6(v6.ip), IpAddr::V6(v6.netmask)),
			};
			IpNet::with_netmask(ip, netmask).ok()
		})
		.collect()
}

#[cfg(target_arch = "wasm32")]
fn local_subnets() -> Vec<IpNet> {
	Vec::new()
}

#[cfg(test)]
mod tests {
	use super::*;

	fn nets(cidrs: &[&str]) -> Vec<IpNet> {
		cidrs.iter().map(|c| c.parse().expect("cidr")).collect()
	}
	fn addr(s: &str) -> Multiaddr {
		s.parse().expect("multiaddr")
	}

	#[test]
	fn public_ipv4_is_dialable() {
		assert!(is_dialable_addr(&addr("/ip4/1.1.1.1/udp/4001/quic-v1"), &[]));
	}

	#[test]
	fn private_ipv4_in_local_subnet_is_dialable() {
		let local = nets(&["192.168.1.5/24"]);
		assert!(is_dialable_addr(&addr("/ip4/192.168.1.42/udp/4001/quic-v1"), &local));
	}

	#[test]
	fn private_ipv4_outside_local_subnets_is_skipped() {
		let local = nets(&["192.168.1.5/24"]);
		assert!(!is_dialable_addr(&addr("/ip4/10.0.0.5/udp/4001/quic-v1"), &local));
	}

	#[test]
	fn private_ipv4_with_no_local_subnets_is_skipped() {
		assert!(!is_dialable_addr(&addr("/ip4/192.168.1.42/udp/4001/quic-v1"), &[]));
	}

	#[test]
	fn loopback_link_local_unspecified_ipv4_are_skipped() {
		let local = nets(&["192.168.1.5/24"]);
		assert!(!is_dialable_addr(&addr("/ip4/127.0.0.1/udp/4001/quic-v1"), &local));
		assert!(!is_dialable_addr(&addr("/ip4/169.254.1.1/udp/4001/quic-v1"), &local));
		assert!(!is_dialable_addr(&addr("/ip4/0.0.0.0/udp/4001/quic-v1"), &local));
	}

	#[test]
	fn cgnat_ipv4_is_subnet_gated() {
		// RFC6598 is not globally routable: skipped when not in our subnets.
		assert!(!is_dialable_addr(&addr("/ip4/100.100.100.100/udp/4001/quic-v1"), &[]));
		// ...but dialable if it is one of our local networks.
		let local = nets(&["100.100.100.1/24"]);
		assert!(is_dialable_addr(&addr("/ip4/100.100.100.42/udp/4001/quic-v1"), &local));
	}

	#[test]
	fn broadcast_and_multicast_ipv4_are_skipped() {
		assert!(!is_dialable_addr(&addr("/ip4/255.255.255.255/udp/4001/quic-v1"), &[]));
		assert!(!is_dialable_addr(&addr("/ip4/224.0.0.1/udp/4001/quic-v1"), &[]));
	}

	#[test]
	fn ietf_protocol_ipv4_is_skipped() {
		// 192.0.0.0/24 (rfc 6890/7335), including the 464xlat clat stub 192.0.0.2, is never dialable.
		assert!(!is_dialable_addr(&addr("/ip4/192.0.0.2/udp/4001/quic-v1"), &[]));
		assert!(!is_dialable_addr(&addr("/ip4/192.0.0.1/udp/4001/quic-v1"), &[]));
	}

	#[test]
	fn circuit_addrs_are_skipped() {
		assert!(!is_dialable_addr(&addr("/ip4/1.1.1.1/tcp/4001/p2p-circuit"), &[]));
	}

	#[test]
	fn ipv6_global_dialable_ula_needs_subnet() {
		assert!(is_dialable_addr(&addr("/ip6/2606:4700:4700::1111/udp/4001/quic-v1"), &[]));
		assert!(!is_dialable_addr(&addr("/ip6/fd00::1/udp/4001/quic-v1"), &[]));
		let local = nets(&["fd00::1/64"]);
		assert!(is_dialable_addr(&addr("/ip6/fd00::42/udp/4001/quic-v1"), &local));
	}

	#[test]
	fn ipv6_loopback_and_link_local_are_skipped() {
		assert!(!is_dialable_addr(&addr("/ip6/::1/udp/4001/quic-v1"), &[]));
		assert!(!is_dialable_addr(&addr("/ip6/fe80::1/udp/4001/quic-v1"), &[]));
	}

	#[test]
	fn dns_addr_without_ip_is_dialable() {
		assert!(is_dialable_addr(&addr("/dns4/example.com/tcp/4001"), &[]));
	}

	#[test]
	fn local_subnets_does_not_panic() {
		// Host-dependent; we only assert it runs and returns owned data.
		let _nets = local_subnets();
	}

	#[test]
	fn addresses_to_dial_filters_and_dedups() {
		let mut task = IdentifyDialNetworkTask::new("co/0.1.0".to_string());
		let peer = PeerId::random();
		let local = nets(&["192.168.1.5/24"]);
		let listen = vec![
			addr("/ip4/192.168.1.42/udp/4001/quic-v1"), // dialable
			addr("/ip4/10.0.0.9/udp/4001/quic-v1"),     // private, not our subnet → skip
			addr("/p2p-circuit"),                       // relay → skip
		];
		let first = task.addresses_to_dial(peer, &listen, &local);
		assert_eq!(first, vec![addr("/ip4/192.168.1.42/udp/4001/quic-v1")]);
		// Repeat: the dialable address is already attempted → nothing new.
		assert!(task.addresses_to_dial(peer, &listen, &local).is_empty());
	}

	#[test]
	fn forget_peer_allows_redial() {
		let mut task = IdentifyDialNetworkTask::new("co/0.1.0".to_string());
		let peer = PeerId::random();
		let local = nets(&["192.168.1.5/24"]);
		let listen = vec![addr("/ip4/192.168.1.42/udp/4001/quic-v1")];
		assert_eq!(task.addresses_to_dial(peer, &listen, &local).len(), 1);
		assert!(task.addresses_to_dial(peer, &listen, &local).is_empty());
		task.forget_peer(&peer);
		assert_eq!(task.addresses_to_dial(peer, &listen, &local).len(), 1);
	}

	#[test]
	fn failed_dial_releases_addresses_for_retry() {
		let mut task = IdentifyDialNetworkTask::new("co/0.1.0".to_string());
		let peer = PeerId::random();
		let local = nets(&["192.168.1.5/24"]);
		let listen = vec![addr("/ip4/192.168.1.42/udp/4001/quic-v1")];
		let to_dial = task.addresses_to_dial(peer, &listen, &local);
		assert_eq!(to_dial.len(), 1);
		let connection_id = ConnectionId::new_unchecked(1);
		task.record_pending(connection_id, peer, to_dial.clone());
		assert!(task.addresses_to_dial(peer, &listen, &local).is_empty());

		// the dial failed: its addresses are released, the next identify retries them.
		assert_eq!(task.dial_failed(&connection_id), Some(peer));
		assert_eq!(task.addresses_to_dial(peer, &listen, &local), to_dial);
	}

	#[test]
	fn established_dial_keeps_addresses_deduplicated() {
		let mut task = IdentifyDialNetworkTask::new("co/0.1.0".to_string());
		let peer = PeerId::random();
		let local = nets(&["192.168.1.5/24"]);
		let listen = vec![addr("/ip4/192.168.1.42/udp/4001/quic-v1")];
		let to_dial = task.addresses_to_dial(peer, &listen, &local);
		let connection_id = ConnectionId::new_unchecked(1);
		task.record_pending(connection_id, peer, to_dial);
		assert!(task.dial_established(&connection_id));

		// established: repeated identifies must not open duplicate connections.
		assert!(task.addresses_to_dial(peer, &listen, &local).is_empty());
	}

	#[test]
	fn rejected_dial_releases_addresses_for_retry() {
		// the swarm rejected the dial synchronously (e.g. a concurrent pending
		// dial): the addresses are released immediately.
		let mut task = IdentifyDialNetworkTask::new("co/0.1.0".to_string());
		let peer = PeerId::random();
		let local = nets(&["192.168.1.5/24"]);
		let listen = vec![addr("/ip4/192.168.1.42/udp/4001/quic-v1")];
		let to_dial = task.addresses_to_dial(peer, &listen, &local);
		task.unmark(&peer, &to_dial);
		assert_eq!(task.addresses_to_dial(peer, &listen, &local), to_dial);
	}

	#[test]
	fn unknown_dial_outcomes_are_noops() {
		let mut task = IdentifyDialNetworkTask::new("co/0.1.0".to_string());
		let connection_id = ConnectionId::new_unchecked(7);
		assert_eq!(task.dial_failed(&connection_id), None);
		assert!(!task.dial_established(&connection_id));
	}

	#[test]
	fn failed_dial_after_full_disconnect_does_not_resurrect_peer() {
		let mut task = IdentifyDialNetworkTask::new("co/0.1.0".to_string());
		let peer = PeerId::random();
		let local = nets(&["192.168.1.5/24"]);
		let listen = vec![addr("/ip4/192.168.1.42/udp/4001/quic-v1")];
		let to_dial = task.addresses_to_dial(peer, &listen, &local);
		let connection_id = ConnectionId::new_unchecked(1);
		task.record_pending(connection_id, peer, to_dial);
		task.forget_peer(&peer);

		// the failure resolves the pending dial without recreating peer state.
		assert_eq!(task.dial_failed(&connection_id), Some(peer));
		assert!(task.dialed.is_empty());
	}
}
