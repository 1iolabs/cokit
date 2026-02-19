// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use co_identity::{network_did_discovery, DidCommHeader, Identity, PeerDidCommHeader, PrivateIdentity};
use co_primitives::{serde_string_enum, to_json_string, NetworkDidDiscovery};
use libp2p::{Multiaddr, PeerId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

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
		message_body: Option<&DiscoverMessage>,
	) -> Result<DidDiscovery, anyhow::Error>
	where
		F: PrivateIdentity + Send + Sync + 'static,
		T: Identity + Send + Sync + 'static,
	{
		let network = network_did_discovery(to, network)?;
		let (from_context, to_context, header) = DidCommHeader::create(from, to, message_type)?;
		let message_header = PeerDidCommHeader { header, from_peer_id: Some(from_peer.to_string()) };
		let message_id = message_header.header.id.clone();
		let message_body = match message_body {
			Some(body) => to_json_string(body)?,
			None => "null".to_owned(),
		};
		let message = from_context.jwe(&to_context, message_header.into(), &message_body)?;
		Ok(DidDiscovery { message_id, network, message })
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum DidDiscoveryMessageType {
	/// Message type for a did discovery request.
	#[serde(rename = "diddiscovery")]
	Discover,

	/// Response message type to an did discovery request.
	#[serde(rename = "diddiscovery-resolve")]
	Resolve,
}
impl DidDiscoveryMessageType {
	pub fn from_str(value: &str) -> Option<Self> {
		Self::try_from(value.to_owned()).ok()
	}
}
serde_string_enum!(DidDiscoveryMessageType);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct DiscoverMessage {
	/// Endpoints where we (our local peer) can be dialed.
	/// This are the callback endpoints when the discovery request is accepted.
	#[serde(rename = "e")]
	pub endpoints: BTreeSet<Multiaddr>,
}
