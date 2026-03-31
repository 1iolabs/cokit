// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::CoId;
use co_macros::co;

/// Network service connectivity description.
#[co]
#[non_exhaustive]
pub enum Network {
	/// DID Discovery protocol.
	//#[serde(rename = "/meshsub/1.1.0/diddiscovery/1.0.0")]
	DidDiscovery(NetworkDidDiscovery),

	/// CO Heads protocol.
	//#[serde(rename = "/meshsub/1.1.0/coheads/1.0.0")]
	CoHeads(NetworkCoHeads),

	/// Rendezvouz protocol.
	//#[serde(rename = "/rendezvous/1.0.0")]
	Rendezvous(NetworkRendezvous),

	/// Direct peer connection.
	//#[serde(rename = "/p2p")]
	Peer(NetworkPeer),
}

/// DID Discovery protocol.
#[co]
pub struct NetworkDidDiscovery {
	/// The GossipSub topic used for DidDiscovery messages.
	/// If not specified the default topic will be used: `"co-contact"`.
	pub topic: Option<String>,

	/// The DID to be discovered.
	pub did: String,
}

/// CO Heads protocol.
#[co]
pub struct NetworkCoHeads {
	/// The GossipSub topic used for Heads messages.
	/// If not specified the default topic will be used: `"co-{co.id}"`.
	pub topic: Option<String>,

	/// The CO to be discovered.
	pub id: CoId,
}

/// Rendezvouz protocol.
#[co]
pub struct NetworkRendezvous {
	/// The namespace to register to.
	pub namespace: String,
	/// Rendezvouz node multi-addresses.
	pub addresses: Vec<String>,
}

/// Direct peer connection.
#[co]
pub struct NetworkPeer {
	/// The [`libp2p::PeerId`] as bytes.
	pub peer: Vec<u8>,
	/// Optional known Multi-addresses.
	/// If empty, mDNS, Kademila and Bluetooth LE may be used to resolve.
	pub addresses: Vec<String>,
}
