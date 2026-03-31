// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

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
pub use settings::{NetworkDns, NetworkSettings};
pub use spawner::CoNetworkTaskSpawner;
pub use subscribe::subscribe_identity;
#[cfg(feature = "native")]
pub use tasks::mdns_gossip::MdnsGossipNetworkTask;
pub use tasks::{
	connections::ConnectionsNetworkTask,
	dial::DialNetworkTask,
	didcomm_receive::DidCommReceiveNetworkTask,
	didcomm_send::DidCommSendNetworkTask,
	discovery::DiscoveryNetworkTask,
	gossip::{
		ListenGossipTask, MeshPeersNetworkTask, PublishGossipTask, PublishGossipTaskError, SubscribeGossipTask,
		UnsubscribeGossipTask,
	},
	listeners::ListnersNetworkTask,
	peers::PeersNetworkTask,
};
