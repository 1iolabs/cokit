pub mod bitswap;
pub mod didcomm;
mod didcontact;
pub mod discovery;
mod library;
mod network;
mod types;

pub use didcontact::{
	create_gossipsub, publish, resolve, subscribe, unsubscribe, Error, RendezvousPoint, ResolveError, ResolveResult,
};
pub use library::{
	backoff::{backoff, backoff_with_jitter},
	clone_key_pair::clone_key_pair,
	find_peer_id::{find_peer_id, try_peer_id},
	static_peer_provider::StaticPeerProvider,
};
pub use network::{Behaviour, Context, Libp2pNetwork, Libp2pNetworkConfig, NetworkEvent, NetworkMode, Shutdown};
pub use types::{
	error::NetworkError,
	layer_behaviour::{Layer, LayerBehaviour},
	layer_provider::DiscoveryLayerBehaviourProvider,
	network_task::{FnOnceNetworkTask, NetworkTask, NetworkTaskBox, NetworkTaskSpawner, TokioNetworkTaskSpawner},
	peer_provider::PeerProvider,
	provider::{
		BitswapBehaviourProvider, DidcommBehaviourProvider, GossipsubBehaviourProvider, MdnsBehaviourProvider,
		RendezvousClientBehaviourProvider,
	},
};
