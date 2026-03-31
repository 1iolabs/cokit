// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

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
