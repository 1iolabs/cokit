pub mod bitswap;
pub mod didcomm;
pub mod discovery;
mod library;
mod network;
pub mod services;
mod types;

pub use library::{
	backoff::{backoff, backoff_with_jitter},
	clone_key_pair::clone_key_pair,
	find_peer_id::{find_peer_id, try_peer_id},
	network_discovery::identities_networks,
	static_peer_provider::StaticPeerProvider,
};
pub use network::{Behaviour, Context, Libp2pNetwork, Libp2pNetworkConfig, NetworkEvent, NetworkMode, Shutdown};
pub use types::{
	error::NetworkError,
	heads::{HeadsErrorCode, HeadsMessage},
	peer_provider::PeerProvider,
};
