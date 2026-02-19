// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use libp2p::{multiaddr::Protocol, Multiaddr, PeerId};

pub fn find_peer_id(addr: &Multiaddr) -> Option<PeerId> {
	addr.iter()
		.filter_map(|item| match item {
			Protocol::P2p(peer) => Some(peer),
			_ => None,
		})
		.next()
}

pub fn try_peer_id(addr: &Multiaddr) -> Result<PeerId, anyhow::Error> {
	find_peer_id(addr).ok_or(anyhow::anyhow!("Invalid address (missing p2p): {}", addr))
}
