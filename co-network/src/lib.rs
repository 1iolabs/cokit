// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

mod bitswap;
mod didcomm;
mod library;
mod network;
mod services;
mod types;

pub use bitswap::{BitswapMessage, Token};
pub use didcomm::EncodedMessage;
pub use library::{
	backoff::{backoff, backoff_with_jitter},
	clone_key_pair::clone_key_pair,
	find_peer_id::{find_peer_id, try_peer_id},
	network_discovery::identities_networks,
	static_peer_provider::StaticPeerProvider,
};
pub use services::{
	heads::HeadsApi,
	network::{subscribe_identity, Network, NetworkApi, NetworkInitialize, NetworkMessage, NetworkSettings},
};
pub use types::{
	error::NetworkError,
	heads::{HeadsErrorCode, HeadsMessage},
	peer_provider::PeerProvider,
};
pub mod connections {
	pub use crate::services::connections::{
		action::*, CoConnection, ConnectionMessage, ConnectionState, Connections, DidConnection,
		DynamicNetworkResolver, NetworkConnection, NetworkResolver, PeerConnection,
	};
}

// external re-exports
pub use libp2p::{identity::Keypair, PeerId};
