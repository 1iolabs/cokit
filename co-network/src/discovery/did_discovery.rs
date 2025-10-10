use co_identity::{network_did_discovery, DidCommHeader, Identity, PeerDidCommHeader, PrivateIdentity};
use co_primitives::{serde_string_enum, NetworkDidDiscovery};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DidDiscovery {
	pub network: NetworkDidDiscovery,
	pub message_id: String,
	pub message: String,
}
impl DidDiscovery {
	/// Create DID Discovery request.
	pub fn create<F, T>(
		from_peer: PeerId,
		from: &F,
		to: &T,
		network: Option<NetworkDidDiscovery>,
		message_type: String,
	) -> Result<DidDiscovery, anyhow::Error>
	where
		F: PrivateIdentity + Send + Sync + 'static,
		T: Identity + Send + Sync + 'static,
	{
		let network = network_did_discovery(to, network)?;
		let (from_context, to_context, header) = DidCommHeader::create(from, to, message_type)?;
		let message_header = PeerDidCommHeader { header, from_peer_id: Some(from_peer.to_string()) };
		let message_id = message_header.header.id.clone();
		let message = from_context.jwe(&to_context, message_header.into(), "null")?;
		Ok(DidDiscovery { message_id, network, message })
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum DidDiscoveryMessage {
	/// Message type for a did discovery request.
	#[serde(rename = "diddiscovery")]
	Discover,

	/// Response message type to an did discovery request.
	#[serde(rename = "diddiscovery-resolve")]
	Resolve,
}
impl DidDiscoveryMessage {
	pub fn from_str(value: &str) -> Option<Self> {
		Self::try_from(value.to_owned()).ok()
	}
}
serde_string_enum!(DidDiscoveryMessage);
