mod behaviour;
mod did_discovery;

pub use behaviour::{ConnectError, Discovery, DiscoveryBehaviour, DiscoveryEvent, DiscoveryState, Event};
pub use did_discovery::{DidDiscovery, DidDiscoveryMessageType, DiscoverMessage};
