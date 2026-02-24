// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

mod actor;
mod api;
mod message;
mod settings;
mod spawner;
mod subscribe;
mod tasks;

pub use actor::{Network, NetworkInitialize};
pub use api::NetworkApi;
pub use message::NetworkMessage;
pub use settings::NetworkSettings;
pub use spawner::CoNetworkTaskSpawner;
pub use subscribe::subscribe_identity;
pub use tasks::{
	connections::ConnectionsNetworkTask,
	dial::DialNetworkTask,
	did_discovery::{DidDiscoverySubscribe, DidDiscoveryUnsubscribe},
	didcomm_receive::DidCommReceiveNetworkTask,
	didcomm_send::DidCommSendNetworkTask,
	discovery_connect::DiscoveryConnectNetworkTask,
	gossip::{ListenGossipTask, PublishGossipTask, SubscribeGossipTask, UnsubscribeGossipTask},
	listeners::ListnersNetworkTask,
	mdns_gossip::MdnsGossipNetworkTask,
	peers::PeersNetworkTask,
};
