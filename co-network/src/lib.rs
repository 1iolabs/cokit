mod bitswap;
mod didcomm;
mod didcontact;
mod heads;
mod library;
mod network;
mod types;

pub use bitswap::{provider::BitswapBehaviourProvider, storage::NetworkBlockStorage};
pub use didcontact::{
	create_gossipsub, publish, resolve, subscribe, unsubscribe, Error, RendezvousPoint, ResolveError, ResolveResult,
};
pub use heads::{Heads, HeadsHandler};
pub use library::clone_key_pair::clone_key_pair;
pub use network::{Behaviour, BehaviourEvent, Libp2pNetwork, Libp2pNetworkConfig, NetworkMode};
pub use types::{
	error::NetworkError,
	network_task::{FnOnceNetworkTask, NetworkTask, NetworkTaskBox, NetworkTaskSpawner},
	provider::{GossipsubBehaviourProvider, PeerProvider},
};
