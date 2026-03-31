// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

#[cfg(feature = "native")]
use crate::services::discovery::action::MdnsDiscoveredAction;
use crate::{
	didcomm,
	network::{Behaviour, NetworkEvent},
	services::discovery::{
		action::{
			DidCommReceivedAction, DiscoveryAction, GossipMessageAction, GossipPeerSubscribedAction,
			GossipPeerUnsubscribedAction, PeerConnectedAction, PeerDisconnectedAction,
		},
		DidDiscoveryMessageType, DiscoveryMessage,
	},
	types::network_task::NetworkTask,
};
use co_actor::ActorHandle;
use libp2p::{gossipsub, swarm::SwarmEvent, Swarm};
#[cfg(feature = "native")]
use std::collections::BTreeSet;

/// Long-lived network task that bridges swarm events to the discovery actor.
#[derive(Debug)]
pub struct DiscoveryNetworkTask {
	handle: ActorHandle<DiscoveryMessage>,
}
impl DiscoveryNetworkTask {
	pub fn new(handle: ActorHandle<DiscoveryMessage>) -> Self {
		Self { handle }
	}
}
impl NetworkTask<Behaviour> for DiscoveryNetworkTask {
	fn execute(&mut self, _swarm: &mut Swarm<Behaviour>) {}

	fn on_swarm_event(
		&mut self,
		_swarm: &mut Swarm<Behaviour>,

		event: SwarmEvent<NetworkEvent>,
	) -> Option<SwarmEvent<NetworkEvent>> {
		match &event {
			// peer connection established (first connection).
			SwarmEvent::ConnectionEstablished { peer_id, num_established, .. } if num_established.get() == 1 => {
				self.handle
					.dispatch(DiscoveryAction::PeerConnected(PeerConnectedAction { peer_id: *peer_id }))
					.ok();
			},

			// peer connection closed (last connection).
			SwarmEvent::ConnectionClosed { peer_id, num_established, .. } if *num_established == 0 => {
				self.handle
					.dispatch(DiscoveryAction::PeerDisconnected(PeerDisconnectedAction { peer_id: *peer_id }))
					.ok();
			},

			// gossipsub events.
			SwarmEvent::Behaviour(NetworkEvent::Gossipsub(gossip_event)) => match gossip_event {
				gossipsub::Event::Message { message, .. } => {
					self.handle
						.dispatch(DiscoveryAction::GossipMessage(GossipMessageAction {
							topic: message.topic.clone(),
							source: message.source,
							data: message.data.clone(),
						}))
						.ok();
				},
				gossipsub::Event::Subscribed { peer_id, topic } => {
					self.handle
						.dispatch(DiscoveryAction::GossipPeerSubscribed(GossipPeerSubscribedAction {
							peer_id: *peer_id,
							topic: topic.clone(),
						}))
						.ok();
				},
				gossipsub::Event::Unsubscribed { peer_id, topic } => {
					self.handle
						.dispatch(DiscoveryAction::GossipPeerUnsubscribed(GossipPeerUnsubscribedAction {
							peer_id: *peer_id,
							topic: topic.clone(),
						}))
						.ok();
				},
				_ => {},
			},

			// didcomm events (filtered for discovery-resolve).
			SwarmEvent::Behaviour(NetworkEvent::Didcomm(didcomm::Event::Received { peer_id, message })) => {
				let message_type = DidDiscoveryMessageType::try_from(message.header().message_type.clone()).ok();
				if message_type == Some(DidDiscoveryMessageType::Resolve) {
					self.handle
						.dispatch(DiscoveryAction::DidCommReceived(DidCommReceivedAction {
							peer_id: *peer_id,
							header: message.header().to_owned(),
						}))
						.ok();
				}
			},

			// mdns events (native only).
			#[cfg(feature = "native")]
			SwarmEvent::Behaviour(NetworkEvent::Mdns(libp2p::mdns::Event::Discovered(items))) => {
				let peers: BTreeSet<_> = items.iter().map(|(peer, _)| *peer).collect();
				if !peers.is_empty() {
					self.handle
						.dispatch(DiscoveryAction::MdnsDiscovered(MdnsDiscoveredAction { peers }))
						.ok();
				}
			},

			_ => {},
		}

		// always forward — never consume events.
		Some(event)
	}

	fn is_complete(&mut self) -> bool {
		self.handle.is_closed()
	}
}
