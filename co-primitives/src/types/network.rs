use serde::{Deserialize, Serialize};

/// Network service connectivity description.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Network {
	/// DID Discovery protocol.
	//#[serde(rename = "/meshsub/1.1.0/diddiscovery/1.0.0")]
	DidDiscovery {
		/// The GossipSub topic used for DidDiscovery messages.
		/// If not specified the default topic will be used: `"co-contact"`.
		topic: Option<String>,
	},

	/// CO Heads protocol.
	//#[serde(rename = "/meshsub/1.1.0/coheads/1.0.0")]
	CoHeads {
		/// The GossipSub topic used for DidDiscovery messages.
		/// If not specified the default topic will be used: `co.id`.
		topic: Option<String>,
	},

	/// Rendezvouz protocol.
	//#[serde(rename = "/rendezvous/1.0.0")]
	Rendezvous {
		/// The namespace to register to.
		namespace: String,
		/// Rendezvouz node multi-addresses.
		addresses: Vec<String>,
	},

	/// Direct peer connection.
	//#[serde(rename = "/p2p")]
	Peer {
		/// The [`libp2p::PeerId`] as bytes.
		peer: Vec<u8>,
		/// Optional known Multi-addresses.
		/// If not specified, mDNS, Kademila and Bluetooth LE may be used to resolve.
		addresses: Vec<String>,
	},
}
