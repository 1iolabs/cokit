mod bitswap;
pub mod didcomm;
mod didcontact;
pub mod discovery;
pub mod heads;
mod library;
mod network;
mod types;

pub use bitswap::storage::NetworkBlockStorage;
pub use didcontact::{
	create_gossipsub, publish, resolve, subscribe, unsubscribe, Error, RendezvousPoint, ResolveError, ResolveResult,
};
pub use discovery::DidDiscoveryMessage;
pub use library::{clone_key_pair::clone_key_pair, static_peer_provider::StaticPeerProvider};
pub use network::{Behaviour, Context, Libp2pNetwork, Libp2pNetworkConfig, NetworkEvent, NetworkMode, Shutdown};
pub use types::{
	error::NetworkError,
	layer_behaviour::{Layer, LayerBehaviour},
	layer_provider::{DiscoveryLayerBehaviourProvider, HeadsLayerBehaviourProvider},
	network_task::{FnOnceNetworkTask, NetworkTask, NetworkTaskBox, NetworkTaskSpawner},
	peer_provider::PeerProvider,
	provider::{
		BitswapBehaviourProvider, DidcommBehaviourProvider, GossipsubBehaviourProvider, MdnsBehaviourProvider,
		RendezvousClientBehaviourProvider,
	},
};
