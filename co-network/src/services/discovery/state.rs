// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use super::action::*;
use crate::services::discovery::{self, DidDiscovery, DidDiscoveryMessageType, Discovery};
use co_actor::Reducer;
use co_identity::{Identity, PrivateIdentityBox};
use co_primitives::NetworkDidDiscovery;
use libp2p::{gossipsub, Multiaddr, PeerId};
use std::{
	collections::{BTreeMap, BTreeSet, VecDeque},
	time::Duration,
};

/// Active subscription listening for DID Discovery requests.
#[derive(Debug, Clone, PartialEq)]
pub enum DidDiscoverySubscription {
	Default,
	Identity(NetworkDidDiscovery, PrivateIdentityBox),
}

/// Request to try to connect peers using supplied discovery methods.
#[derive(Debug, Clone)]
pub struct DiscoveryConnectRequest {
	pub id: u64,
	pub discovery: BTreeSet<Discovery>,
	/// Cache for all direct PeerId we are interested in.
	pub discovery_peers: BTreeSet<PeerId>,
	pub timeout: Duration,
	pub max_peers: Option<u16>,
	pub connected_peers: BTreeSet<PeerId>,
	pub span: tracing::Span,
}
impl DiscoveryConnectRequest {
	pub fn is_max_peers(&self) -> bool {
		matches!((self.max_peers, self.connected_peers.len()), (Some(max), len) if len >= max as usize)
	}

	pub fn build(&mut self) -> Result<(), anyhow::Error> {
		for item in &self.discovery {
			item.validate()?;
		}
		self.build_discovery_peers()?;
		Ok(())
	}

	fn build_discovery_peers(&mut self) -> Result<(), anyhow::Error> {
		self.discovery_peers = self
			.discovery
			.iter()
			.filter_map(|discovery| match discovery {
				Discovery::Peer(network) => Some(PeerId::from_bytes(&network.peer)),
				_ => None,
			})
			.collect::<Result<_, _>>()?;
		Ok(())
	}
}

/// Discovery service state. No swarm references, no generic resolver.
#[derive(Debug)]
pub struct DiscoveryState {
	/// Our PeerId.
	pub local_peer_id: PeerId,
	/// Next discovery request id.
	pub next_id: u64,
	/// Active discovery requests.
	pub requests: BTreeMap<u64, DiscoveryConnectRequest>,
	/// Active subscription listening for DID Discovery requests.
	pub did_subscriptions: BTreeMap<gossipsub::TopicHash, Vec<DidDiscoverySubscription>>,
	/// Pending DID Discovery requests. Insufficient peers.
	pub pending_discovery: VecDeque<(u64, gossipsub::TopicHash, DidDiscovery)>,
	/// Default discovery timeout.
	pub timeout: Duration,
	/// Default discovery max peers.
	pub max_peers: Option<u16>,
	/// Currently connected peers.
	pub connected_peers: BTreeSet<PeerId>,
}
impl DiscoveryState {
	pub fn allocate_id(&mut self) -> u64 {
		let id = self.next_id;
		self.next_id += 1;
		id
	}
}
impl Reducer<DiscoveryAction> for DiscoveryState {
	fn reduce(&mut self, action: DiscoveryAction) -> Vec<DiscoveryAction> {
		match action {
			DiscoveryAction::Connect(action) => self.reduce_connect(action),
			DiscoveryAction::Release(action) => self.reduce_release(action),
			DiscoveryAction::DidSubscribe(action) => self.reduce_did_subscribe(action),
			DiscoveryAction::DidUnsubscribe(action) => self.reduce_did_unsubscribe(action),
			DiscoveryAction::PeerConnected(action) => self.reduce_peer_connected(action),
			DiscoveryAction::PeerDisconnected(action) => self.reduce_peer_disconnected(action),
			DiscoveryAction::GossipPeerSubscribed(action) => self.reduce_gossip_peer_subscribed(action),
			DiscoveryAction::GossipPeerUnsubscribed(action) => self.reduce_gossip_peer_unsubscribed(action),
			DiscoveryAction::DidCommReceived(action) => self.reduce_didcomm_received(action),
			DiscoveryAction::MdnsDiscovered(action) => self.reduce_mdns_discovered(action),
			DiscoveryAction::DidPublishPending(action) => self.reduce_did_publish_pending(action),
			DiscoveryAction::MeshPeersResult(action) => self.reduce_mesh_peers_result(action),
			DiscoveryAction::DidDecrypted(action) => self.reduce_did_decrypted(action),
			DiscoveryAction::Timeout(action) => self.reduce_timeout(action),
			// actions handled by epics only — no state change.
			DiscoveryAction::GossipMessage(_)
			| DiscoveryAction::GossipSubscribe(_)
			| DiscoveryAction::GossipUnsubscribe(_)
			| DiscoveryAction::DidPublish(_)
			| DiscoveryAction::QueryMeshPeers(_)
			| DiscoveryAction::DialPeer(_)
			| DiscoveryAction::SendResolve(_)
			| DiscoveryAction::Event(_) => vec![],
		}
	}
}
impl DiscoveryState {
	fn reduce_connect(&mut self, action: ConnectAction) -> Vec<DiscoveryAction> {
		let id = action.id;
		let span = tracing::trace_span!("discovery", discovery_id = id);
		let _enter = span.enter();

		let mut request = DiscoveryConnectRequest {
			id,
			discovery: action.discovery,
			max_peers: self.max_peers,
			timeout: self.timeout,
			discovery_peers: Default::default(),
			connected_peers: Default::default(),
			span: span.clone(),
		};

		if let Err(err) = request.build() {
			tracing::warn!(?err, "discovery-connect-build-failed");
			return vec![];
		}

		tracing::trace!(timeout = ?request.timeout, discovery = ?request.discovery, "discovery");
		self.requests.insert(id, request);

		let mut actions = self.try_connect(id);

		// check if any requested peers are already connected.
		if let Some(request) = self.requests.get_mut(&id) {
			for peer in request.discovery_peers.clone() {
				if self.connected_peers.contains(&peer) && request.connected_peers.insert(peer) {
					actions.push(DiscoveryAction::Event(discovery::Event::Connected { id, peer }));
				}
			}
		}

		actions
	}

	fn try_connect(&mut self, request_id: u64) -> Vec<DiscoveryAction> {
		let request = match self.requests.get(&request_id) {
			Some(r) => r,
			None => return vec![],
		};

		let mut actions = Vec::new();
		let mut discovery_used = 0;

		for item in request.discovery.clone() {
			match item {
				Discovery::DidDiscovery(did_disc) => {
					let topic = did_discovery_topic(&did_disc.network);
					if !self.did_subscriptions.contains_key(&topic.hash()) {
						tracing::trace!(network = ?did_disc.network, ?topic, "discovery-did-unsubscribed");
						continue;
					}
					actions.push(DiscoveryAction::DidPublish(DidPublishAction { request_id, discovery: did_disc }));
				},
				Discovery::Topic(topic_str) => {
					actions
						.push(DiscoveryAction::QueryMeshPeers(QueryMeshPeersAction { request_id, topic: topic_str }));
				},
				Discovery::Rendezvous(_) => {
					// TODO: implement
					continue;
				},
				Discovery::Peer(peer) => {
					let peer_id = match PeerId::from_bytes(&peer.peer) {
						Ok(p) => p,
						Err(_) => continue,
					};
					if peer_id == self.local_peer_id {
						continue;
					}
					let addresses: Vec<Multiaddr> =
						peer.addresses.iter().filter_map(|a| a.parse::<Multiaddr>().ok()).collect();
					actions.push(DiscoveryAction::DialPeer(DialPeerAction {
						request_id: Some(request_id),
						peer_id,
						addresses,
					}));
				},
			}
			discovery_used += 1;
		}

		if discovery_used == 0 {
			// No network usable. The event stream will just not receive Connected events,
			// and eventually the request will time out.
			tracing::trace!(request_id, "discovery-no-network");
		}

		actions
	}

	fn reduce_release(&mut self, action: ReleaseAction) -> Vec<DiscoveryAction> {
		tracing::trace!(
			parent: self.requests.get(&action.id).and_then(|s| s.span.id()),
			"discovery-release"
		);
		self.pending_discovery.retain(|(request, _, _)| *request != action.id);
		self.requests.remove(&action.id);
		vec![]
	}

	fn reduce_did_subscribe(&mut self, action: DidSubscribeAction) -> Vec<DiscoveryAction> {
		let topic = gossipsub::IdentTopic::new(&action.topic_str);
		self.did_subscriptions
			.entry(topic.hash())
			.or_default()
			.push(action.subscription);

		let count = self.did_subscriptions.get(&topic.hash()).map(|v| v.len()).unwrap_or(0);
		if count == 1 {
			return vec![DiscoveryAction::GossipSubscribe(GossipSubscribeAction { topic: action.topic_str })];
		}
		vec![]
	}

	fn reduce_did_unsubscribe(&mut self, action: DidUnsubscribeAction) -> Vec<DiscoveryAction> {
		let (subscription, topic_str) = match &action {
			DidUnsubscribeAction::Identity(did) => {
				let found = self
					.did_subscriptions
					.iter()
					.flat_map(|(_, subs)| subs.iter())
					.find(
						|s| matches!(s, DidDiscoverySubscription::Identity(_, identity) if identity.identity() == did),
					)
					.cloned();
				match found {
					Some(sub) => {
						let topic = did_discovery_subscription_topic_str(&sub).to_owned();
						(sub, topic)
					},
					None => return vec![],
				}
			},
			DidUnsubscribeAction::Default => {
				(DidDiscoverySubscription::Default, did_discovery_topic_default_str().to_owned())
			},
		};

		let topic = gossipsub::IdentTopic::new(&topic_str);
		let removed = if let Some(subscriptions) = self.did_subscriptions.get_mut(&topic.hash()) {
			if let Some((index, _)) = subscriptions.iter().enumerate().find(|(_, item)| item == &&subscription) {
				Some(subscriptions.remove(index))
			} else {
				None
			}
		} else {
			None
		};

		if removed.is_some() {
			// clear empty entry.
			if self.did_subscriptions.get(&topic.hash()).map(|s| s.is_empty()).unwrap_or(false) {
				self.did_subscriptions.remove(&topic.hash());
			}

			// only unsubscribe from gossipsub when no more subscriptions for this topic
			if !self.did_subscriptions.contains_key(&topic.hash()) {
				return vec![DiscoveryAction::GossipUnsubscribe(GossipUnsubscribeAction { topic: topic_str })];
			}
		}

		vec![]
	}

	fn reduce_peer_connected(&mut self, action: PeerConnectedAction) -> Vec<DiscoveryAction> {
		self.connected_peers.insert(action.peer_id);

		let request_ids: Vec<u64> = self
			.requests
			.iter()
			.filter(|(_, r)| r.discovery_peers.contains(&action.peer_id))
			.map(|(id, _)| *id)
			.collect();

		let mut actions = Vec::new();
		for request_id in request_ids {
			if let Some(request) = self.requests.get_mut(&request_id) {
				if request.connected_peers.insert(action.peer_id) {
					tracing::trace!(parent: request.span.id(), peer = ?action.peer_id, "discovery-connected");
					actions.push(DiscoveryAction::Event(discovery::Event::Connected {
						id: request_id,
						peer: action.peer_id,
					}));
				}
			}
		}
		actions
	}

	fn reduce_peer_disconnected(&mut self, action: PeerDisconnectedAction) -> Vec<DiscoveryAction> {
		self.connected_peers.remove(&action.peer_id);

		let request_ids: Vec<u64> = self
			.requests
			.iter()
			.filter(|(_, r)| r.discovery_peers.contains(&action.peer_id))
			.map(|(id, _)| *id)
			.collect();

		let mut actions = Vec::new();
		for request_id in request_ids {
			if let Some(request) = self.requests.get_mut(&request_id) {
				if request.connected_peers.remove(&action.peer_id) {
					tracing::trace!(parent: request.span.id(), peer = ?action.peer_id, "discovery-disconnected");
					actions.push(DiscoveryAction::Event(discovery::Event::Disconnected {
						id: request_id,
						peer: action.peer_id,
					}));
				}
			}
		}
		actions
	}

	fn reduce_gossip_peer_subscribed(&mut self, action: GossipPeerSubscribedAction) -> Vec<DiscoveryAction> {
		if action.peer_id == self.local_peer_id {
			return vec![];
		}

		let mut actions = Vec::new();

		// move pending discoveries for this topic to publish.
		let mut indices_to_remove = Vec::new();
		for (index, (_, pending_topic, _)) in self.pending_discovery.iter().enumerate() {
			if pending_topic == &action.topic {
				indices_to_remove.push(index);
			}
		}
		// remove in reverse order to preserve indices.
		for index in indices_to_remove.into_iter().rev() {
			if let Some((_request, _topic, discovery)) = self.pending_discovery.remove(index) {
				actions.push(DiscoveryAction::DidPublish(DidPublishAction {
					request_id: 0, // retry — request_id not tracked in pending_discovery
					discovery,
				}));
			}
		}

		// topic discovery: dispatch connected events for subscribing peers.
		let topic_str = action.topic.as_str();
		let request_ids: Vec<u64> = self
			.requests
			.iter()
			.filter(|(_, r)| {
				r.discovery
					.iter()
					.any(|d| matches!(d, Discovery::Topic(t) if t.as_str() == topic_str))
			})
			.map(|(id, _)| *id)
			.collect();
		for request_id in request_ids {
			if let Some(request) = self.requests.get_mut(&request_id) {
				if request.connected_peers.insert(action.peer_id) {
					actions.push(DiscoveryAction::Event(discovery::Event::Connected {
						id: request_id,
						peer: action.peer_id,
					}));
				}
			}
		}

		actions
	}

	fn reduce_gossip_peer_unsubscribed(&mut self, action: GossipPeerUnsubscribedAction) -> Vec<DiscoveryAction> {
		if action.peer_id == self.local_peer_id {
			return vec![];
		}

		let topic_str = action.topic.as_str();
		let request_ids: Vec<u64> = self
			.requests
			.iter()
			.filter(|(_, r)| {
				r.discovery
					.iter()
					.any(|d| matches!(d, Discovery::Topic(t) if t.as_str() == topic_str))
			})
			.map(|(id, _)| *id)
			.collect();

		let mut actions = Vec::new();
		for request_id in request_ids {
			if let Some(request) = self.requests.get_mut(&request_id) {
				if request.connected_peers.remove(&action.peer_id) {
					actions.push(DiscoveryAction::Event(discovery::Event::Disconnected {
						id: request_id,
						peer: action.peer_id,
					}));
				}
			}
		}
		actions
	}

	fn reduce_didcomm_received(&mut self, action: DidCommReceivedAction) -> Vec<DiscoveryAction> {
		let message_type = DidDiscoveryMessageType::from_str(&action.header.message_type);
		if message_type != Some(DidDiscoveryMessageType::Resolve) {
			return vec![];
		}

		let request_ids: Vec<u64> = self
			.requests
			.iter()
			.filter(|(_, r)| {
				r.discovery.iter().any(
					|d| matches!(d, Discovery::DidDiscovery(disc) if action.header.thid.as_ref() == Some(&disc.message_id)),
				)
			})
			.map(|(id, _)| *id)
			.collect();

		let mut actions = Vec::new();
		for request_id in request_ids {
			if let Some(request) = self.requests.get_mut(&request_id) {
				if !request.connected_peers.contains(&action.peer_id) {
					request.connected_peers.insert(action.peer_id);
					actions.push(DiscoveryAction::Event(discovery::Event::Connected {
						id: request_id,
						peer: action.peer_id,
					}));
				}
			}
		}
		actions
	}

	fn reduce_mdns_discovered(&mut self, action: MdnsDiscoveredAction) -> Vec<DiscoveryAction> {
		let mut actions = Vec::new();

		for (request, peer_id) in self.all_discovery_peers() {
			if !request.connected_peers.contains(&peer_id) && !request.is_max_peers() && action.peers.contains(&peer_id)
			{
				actions.push(DiscoveryAction::DialPeer(DialPeerAction {
					request_id: Some(request.id),
					peer_id,
					addresses: vec![],
				}));
			}
		}

		actions
	}

	fn reduce_did_publish_pending(&mut self, action: DidPublishPendingAction) -> Vec<DiscoveryAction> {
		self.pending_discovery
			.push_back((action.request_id, action.topic, action.discovery));
		vec![DiscoveryAction::Event(discovery::Event::InsufficentPeers { id: action.request_id })]
	}

	fn reduce_mesh_peers_result(&mut self, action: MeshPeersResultAction) -> Vec<DiscoveryAction> {
		let request = match self.requests.get_mut(&action.request_id) {
			Some(r) => r,
			None => return vec![],
		};

		let mut actions = Vec::new();
		for peer in action.peers {
			if request.connected_peers.insert(peer) {
				actions.push(DiscoveryAction::Event(discovery::Event::Connected { id: action.request_id, peer }));
			}
		}
		actions
	}

	fn reduce_did_decrypted(&mut self, action: DidDecryptedAction) -> Vec<DiscoveryAction> {
		// a DID discovery message was decrypted. Send the resolve response.
		let should_dial = !self.connected_peers.contains(&action.from_peer) && !action.from_endpoints.is_empty();
		let mut actions = vec![DiscoveryAction::SendResolve(SendResolveAction {
			from_peer: action.from_peer,
			from_endpoints: action.from_endpoints.clone(),
			response: action.response,
		})];
		if should_dial {
			actions.push(DiscoveryAction::DialPeer(DialPeerAction {
				request_id: None,
				peer_id: action.from_peer,
				addresses: action.from_endpoints.into_iter().collect(),
			}));
		}
		actions
	}

	fn reduce_timeout(&mut self, action: TimeoutAction) -> Vec<DiscoveryAction> {
		self.pending_discovery.retain(|(request, _, _)| *request != action.id);
		self.requests.remove(&action.id);
		vec![DiscoveryAction::Event(discovery::Event::Timeout { id: action.id })]
	}

	fn all_discovery_peers(&self) -> impl Iterator<Item = (&DiscoveryConnectRequest, PeerId)> {
		self.requests
			.iter()
			.flat_map(|(_, r)| r.discovery_peers.iter().map(move |p| (r, *p)))
	}
}

pub fn did_discovery_topic(network: &NetworkDidDiscovery) -> gossipsub::IdentTopic {
	gossipsub::IdentTopic::new(did_discovery_topic_str(network))
}

pub fn did_discovery_topic_str(network: &NetworkDidDiscovery) -> &str {
	network.topic.as_deref().unwrap_or("co-contact")
}

pub fn did_discovery_subscription_topic_str(subscription: &DidDiscoverySubscription) -> &str {
	match subscription {
		DidDiscoverySubscription::Default => did_discovery_topic_default_str(),
		DidDiscoverySubscription::Identity(network, _) => {
			network.topic.as_deref().unwrap_or(did_discovery_topic_default_str())
		},
	}
}

pub fn did_discovery_topic_default_str() -> &'static str {
	"co-contact"
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::services::discovery::{DidDiscovery, DidDiscoveryMessageType, Discovery, Event};
	use co_actor::Reducer;
	use co_primitives::{NetworkDidDiscovery, NetworkPeer};
	use libp2p::{identity::Keypair, PeerId};
	use std::collections::BTreeSet;

	fn test_peer(seed: u8) -> PeerId {
		let keypair = Keypair::ed25519_from_bytes([seed; 32]).unwrap();
		keypair.public().to_peer_id()
	}

	fn new_state(local_peer: PeerId) -> DiscoveryState {
		DiscoveryState {
			local_peer_id: local_peer,
			next_id: 1,
			requests: Default::default(),
			did_subscriptions: Default::default(),
			pending_discovery: Default::default(),
			timeout: Duration::from_secs(10),
			max_peers: None,
			connected_peers: Default::default(),
		}
	}

	fn peer_discovery(peer: PeerId, addresses: Vec<String>) -> Discovery {
		Discovery::Peer(NetworkPeer { peer: peer.to_bytes(), addresses })
	}

	fn did_discovery(message_id: &str) -> DidDiscovery {
		DidDiscovery {
			network: NetworkDidDiscovery { topic: None, did: "did:key:test".to_owned() },
			message_id: message_id.to_owned(),
			message: "encrypted-message".to_owned(),
		}
	}

	fn connect(state: &mut DiscoveryState, discovery: Vec<Discovery>) -> (u64, Vec<DiscoveryAction>) {
		let id = state.allocate_id();
		let actions =
			state.reduce(DiscoveryAction::Connect(ConnectAction { id, discovery: discovery.into_iter().collect() }));
		(id, actions)
	}

	#[test]
	fn test_peer_connect_emits_dial() {
		let local = test_peer(0);
		let remote = test_peer(1);
		let mut state = new_state(local);

		let (id, actions) = connect(&mut state, vec![peer_discovery(remote, vec!["/ip4/127.0.0.1/tcp/1234".into()])]);

		assert_eq!(state.requests.len(), 1);
		assert!(state.requests.contains_key(&id));

		// should emit DialPeer for the remote peer.
		let dial = actions.iter().find(|a| matches!(a, DiscoveryAction::DialPeer(_)));
		assert!(dial.is_some(), "expected DialPeer action");
		if let DiscoveryAction::DialPeer(dial) = dial.unwrap() {
			assert_eq!(dial.peer_id, remote);
			assert_eq!(dial.request_id, Some(id));
		}
	}

	#[test]
	fn test_peer_connected_emits_event() {
		let local = test_peer(0);
		let remote = test_peer(1);
		let mut state = new_state(local);

		let (id, _) = connect(&mut state, vec![peer_discovery(remote, vec!["/ip4/127.0.0.1/tcp/1234".into()])]);

		// simulate peer connected.
		let actions = state.reduce(DiscoveryAction::PeerConnected(PeerConnectedAction { peer_id: remote }));

		assert!(state.connected_peers.contains(&remote));
		let event = actions
			.iter()
			.find(|a| matches!(a, DiscoveryAction::Event(Event::Connected { .. })));
		assert!(event.is_some(), "expected Connected event");
		if let DiscoveryAction::Event(Event::Connected { id: eid, peer }) = event.unwrap() {
			assert_eq!(*eid, id);
			assert_eq!(*peer, remote);
		}
	}

	#[test]
	fn test_peer_already_connected() {
		let local = test_peer(0);
		let remote = test_peer(1);
		let mut state = new_state(local);

		// remote is already connected before the connect request.
		state.connected_peers.insert(remote);

		let (id, actions) = connect(&mut state, vec![peer_discovery(remote, vec!["/ip4/127.0.0.1/tcp/1234".into()])]);

		// should still emit Connected event for the already-connected peer.
		let event = actions
			.iter()
			.find(|a| matches!(a, DiscoveryAction::Event(Event::Connected { .. })));
		assert!(event.is_some(), "expected Connected event for already-connected peer");
		if let DiscoveryAction::Event(Event::Connected { id: eid, peer }) = event.unwrap() {
			assert_eq!(*eid, id);
			assert_eq!(*peer, remote);
		}
	}

	#[test]
	fn test_peer_disconnected_emits_event() {
		let local = test_peer(0);
		let remote = test_peer(1);
		let mut state = new_state(local);

		let (id, _) = connect(&mut state, vec![peer_discovery(remote, vec!["/ip4/127.0.0.1/tcp/1234".into()])]);
		state.reduce(DiscoveryAction::PeerConnected(PeerConnectedAction { peer_id: remote }));

		let actions = state.reduce(DiscoveryAction::PeerDisconnected(PeerDisconnectedAction { peer_id: remote }));

		assert!(!state.connected_peers.contains(&remote));
		let event = actions
			.iter()
			.find(|a| matches!(a, DiscoveryAction::Event(Event::Disconnected { .. })));
		assert!(event.is_some(), "expected Disconnected event");
		if let DiscoveryAction::Event(Event::Disconnected { id: eid, peer }) = event.unwrap() {
			assert_eq!(*eid, id);
			assert_eq!(*peer, remote);
		}
	}

	#[test]
	fn test_skips_local_peer() {
		let local = test_peer(0);
		let mut state = new_state(local);

		let (_, actions) = connect(&mut state, vec![peer_discovery(local, vec!["/ip4/127.0.0.1/tcp/1234".into()])]);

		// should NOT emit DialPeer for local peer.
		let dial = actions.iter().find(|a| matches!(a, DiscoveryAction::DialPeer(_)));
		assert!(dial.is_none(), "should not dial local peer");
	}

	#[test]
	fn test_timeout_emits_event_and_cleans_up() {
		let local = test_peer(0);
		let remote = test_peer(1);
		let mut state = new_state(local);

		let (id, _) = connect(&mut state, vec![peer_discovery(remote, vec!["/ip4/127.0.0.1/tcp/1234".into()])]);
		assert!(state.requests.contains_key(&id));

		let actions = state.reduce(DiscoveryAction::Timeout(TimeoutAction { id }));

		assert!(!state.requests.contains_key(&id));
		let event = actions
			.iter()
			.find(|a| matches!(a, DiscoveryAction::Event(Event::Timeout { .. })));
		assert!(event.is_some(), "expected Timeout event");
		if let DiscoveryAction::Event(Event::Timeout { id: eid }) = event.unwrap() {
			assert_eq!(*eid, id);
		}
	}

	#[test]
	fn test_release_cleans_up() {
		let local = test_peer(0);
		let remote = test_peer(1);
		let mut state = new_state(local);

		let (id, _) = connect(&mut state, vec![peer_discovery(remote, vec!["/ip4/127.0.0.1/tcp/1234".into()])]);
		assert!(state.requests.contains_key(&id));

		state.reduce(DiscoveryAction::Release(ReleaseAction { id }));

		assert!(!state.requests.contains_key(&id));
	}

	#[test]
	fn test_did_subscribe_emits_gossip_subscribe() {
		let local = test_peer(0);
		let mut state = new_state(local);

		let actions = state.reduce(DiscoveryAction::DidSubscribe(DidSubscribeAction {
			subscription: DidDiscoverySubscription::Default,
			topic_str: "co-contact".to_owned(),
		}));

		assert_eq!(state.did_subscriptions.len(), 1);
		let gossip = actions.iter().find(|a| matches!(a, DiscoveryAction::GossipSubscribe(_)));
		assert!(gossip.is_some(), "expected GossipSubscribe on first subscription");
	}

	#[test]
	fn test_did_subscribe_second_no_gossip() {
		let local = test_peer(0);
		let mut state = new_state(local);

		state.reduce(DiscoveryAction::DidSubscribe(DidSubscribeAction {
			subscription: DidDiscoverySubscription::Default,
			topic_str: "co-contact".to_owned(),
		}));

		// second subscription for same topic should NOT emit GossipSubscribe.
		let actions = state.reduce(DiscoveryAction::DidSubscribe(DidSubscribeAction {
			subscription: DidDiscoverySubscription::Default,
			topic_str: "co-contact".to_owned(),
		}));

		let gossip = actions.iter().find(|a| matches!(a, DiscoveryAction::GossipSubscribe(_)));
		assert!(gossip.is_none(), "should not re-subscribe to gossip");
	}

	#[test]
	fn test_did_unsubscribe_last_emits_gossip_unsubscribe() {
		let local = test_peer(0);
		let mut state = new_state(local);

		state.reduce(DiscoveryAction::DidSubscribe(DidSubscribeAction {
			subscription: DidDiscoverySubscription::Default,
			topic_str: "co-contact".to_owned(),
		}));

		let actions = state.reduce(DiscoveryAction::DidUnsubscribe(DidUnsubscribeAction::Default));

		assert!(state.did_subscriptions.is_empty());
		let gossip = actions.iter().find(|a| matches!(a, DiscoveryAction::GossipUnsubscribe(_)));
		assert!(gossip.is_some(), "expected GossipUnsubscribe on last unsubscription");
	}

	#[test]
	fn test_did_connect_emits_publish() {
		let local = test_peer(0);
		let mut state = new_state(local);

		// subscribe first so the topic is known.
		state.reduce(DiscoveryAction::DidSubscribe(DidSubscribeAction {
			subscription: DidDiscoverySubscription::Default,
			topic_str: "co-contact".to_owned(),
		}));

		let disc = did_discovery("msg-1");
		let (id, actions) = connect(&mut state, vec![Discovery::DidDiscovery(disc.clone())]);

		let publish = actions.iter().find(|a| matches!(a, DiscoveryAction::DidPublish(_)));
		assert!(publish.is_some(), "expected DidPublish action");
		if let DiscoveryAction::DidPublish(p) = publish.unwrap() {
			assert_eq!(p.request_id, id);
			assert_eq!(p.discovery.message_id, "msg-1");
		}
	}

	#[test]
	fn test_did_connect_without_subscription_skips() {
		let local = test_peer(0);
		let mut state = new_state(local);

		// do NOT subscribe — connect should skip DidPublish.
		let disc = did_discovery("msg-1");
		let (_id, actions) = connect(&mut state, vec![Discovery::DidDiscovery(disc)]);

		let publish = actions.iter().find(|a| matches!(a, DiscoveryAction::DidPublish(_)));
		assert!(publish.is_none(), "should not publish without subscription");
	}

	#[test]
	fn test_did_publish_pending_then_retry_on_gossip_peer() {
		let local = test_peer(0);
		let remote = test_peer(1);
		let mut state = new_state(local);

		// subscribe.
		state.reduce(DiscoveryAction::DidSubscribe(DidSubscribeAction {
			subscription: DidDiscoverySubscription::Default,
			topic_str: "co-contact".to_owned(),
		}));

		let disc = did_discovery("msg-1");
		let topic = gossipsub::IdentTopic::new("co-contact");

		// simulate publish failing → pending.
		let actions = state.reduce(DiscoveryAction::DidPublishPending(DidPublishPendingAction {
			request_id: 1,
			topic: topic.hash(),
			discovery: disc.clone(),
		}));

		// should emit InsufficentPeers event.
		let event = actions
			.iter()
			.find(|a| matches!(a, DiscoveryAction::Event(Event::InsufficentPeers { .. })));
		assert!(event.is_some(), "expected InsufficentPeers event");
		assert_eq!(state.pending_discovery.len(), 1);

		// a peer subscribes to the topic → should retry publish
		let actions = state.reduce(DiscoveryAction::GossipPeerSubscribed(GossipPeerSubscribedAction {
			peer_id: remote,
			topic: topic.hash(),
		}));

		assert!(state.pending_discovery.is_empty());
		let publish = actions.iter().find(|a| matches!(a, DiscoveryAction::DidPublish(_)));
		assert!(publish.is_some(), "expected DidPublish retry");
	}

	#[test]
	fn test_did_decrypted_emits_send_resolve_and_dial() {
		let local = test_peer(0);
		let remote = test_peer(1);
		let mut state = new_state(local);

		let addr: Multiaddr = "/ip4/127.0.0.1/tcp/5678".parse().unwrap();
		let mut endpoints = BTreeSet::new();
		endpoints.insert(addr.clone());

		let actions = state.reduce(DiscoveryAction::DidDecrypted(DidDecryptedAction {
			from_peer: remote,
			from_endpoints: endpoints,
			response: "resolve-response".to_owned(),
		}));

		// should emit SendResolve + DialPeer (since remote is not connected).
		let send = actions.iter().find(|a| matches!(a, DiscoveryAction::SendResolve(_)));
		assert!(send.is_some(), "expected SendResolve action");

		let dial = actions.iter().find(|a| matches!(a, DiscoveryAction::DialPeer(_)));
		assert!(dial.is_some(), "expected DialPeer action");
		if let DiscoveryAction::DialPeer(d) = dial.unwrap() {
			assert_eq!(d.peer_id, remote);
			assert!(d.addresses.contains(&addr));
		}
	}

	#[test]
	fn test_did_decrypted_already_connected_skips_dial() {
		let local = test_peer(0);
		let remote = test_peer(1);
		let mut state = new_state(local);
		state.connected_peers.insert(remote);

		let addr: Multiaddr = "/ip4/127.0.0.1/tcp/5678".parse().unwrap();
		let mut endpoints = BTreeSet::new();
		endpoints.insert(addr);

		let actions = state.reduce(DiscoveryAction::DidDecrypted(DidDecryptedAction {
			from_peer: remote,
			from_endpoints: endpoints,
			response: "resolve-response".to_owned(),
		}));

		// sendResolve should still fire, but no DialPeer.
		let send = actions.iter().find(|a| matches!(a, DiscoveryAction::SendResolve(_)));
		assert!(send.is_some(), "expected SendResolve");
		let dial = actions.iter().find(|a| matches!(a, DiscoveryAction::DialPeer(_)));
		assert!(dial.is_none(), "should not dial already-connected peer");
	}

	#[test]
	fn test_didcomm_resolve_emits_connected() {
		let local = test_peer(0);
		let remote = test_peer(1);
		let mut state = new_state(local);

		let disc = did_discovery("msg-1");

		// subscribe + Connect.
		state.reduce(DiscoveryAction::DidSubscribe(DidSubscribeAction {
			subscription: DidDiscoverySubscription::Default,
			topic_str: "co-contact".to_owned(),
		}));
		let (id, _) = connect(&mut state, vec![Discovery::DidDiscovery(disc)]);

		// receive a didcomm resolve response with matching thid
		let actions = state.reduce(DiscoveryAction::DidCommReceived(DidCommReceivedAction {
			peer_id: remote,
			header: co_identity::DidCommHeader {
				id: "resp-1".to_owned(),
				message_type: DidDiscoveryMessageType::Resolve.to_string(),
				thid: Some("msg-1".to_owned()),
				..Default::default()
			},
		}));

		let event = actions
			.iter()
			.find(|a| matches!(a, DiscoveryAction::Event(Event::Connected { .. })));
		assert!(event.is_some(), "expected Connected event on DIDComm resolve");
		if let DiscoveryAction::Event(Event::Connected { id: eid, peer }) = event.unwrap() {
			assert_eq!(*eid, id);
			assert_eq!(*peer, remote);
		}
	}

	#[test]
	fn test_topic_connect_emits_query_mesh_peers() {
		let local = test_peer(0);
		let mut state = new_state(local);

		let (id, actions) = connect(&mut state, vec![Discovery::Topic("my-topic".into())]);

		let query = actions.iter().find(|a| matches!(a, DiscoveryAction::QueryMeshPeers(_)));
		assert!(query.is_some(), "expected QueryMeshPeers action");
		if let DiscoveryAction::QueryMeshPeers(q) = query.unwrap() {
			assert_eq!(q.request_id, id);
			assert_eq!(q.topic, "my-topic");
		}
	}

	#[test]
	fn test_mesh_peers_result_emits_connected() {
		let local = test_peer(0);
		let remote = test_peer(1);
		let mut state = new_state(local);

		let (id, _) = connect(&mut state, vec![Discovery::Topic("my-topic".into())]);

		let actions = state
			.reduce(DiscoveryAction::MeshPeersResult(MeshPeersResultAction { request_id: id, peers: vec![remote] }));

		let event = actions
			.iter()
			.find(|a| matches!(a, DiscoveryAction::Event(Event::Connected { .. })));
		assert!(event.is_some(), "expected Connected event from mesh peers");
		if let DiscoveryAction::Event(Event::Connected { id: eid, peer }) = event.unwrap() {
			assert_eq!(*eid, id);
			assert_eq!(*peer, remote);
		}
	}

	#[test]
	fn test_gossip_peer_subscribed_emits_connected_for_topic() {
		let local = test_peer(0);
		let remote = test_peer(1);
		let mut state = new_state(local);

		let (id, _) = connect(&mut state, vec![Discovery::Topic("my-topic".into())]);

		let topic = gossipsub::IdentTopic::new("my-topic");
		let actions = state.reduce(DiscoveryAction::GossipPeerSubscribed(GossipPeerSubscribedAction {
			peer_id: remote,
			topic: topic.hash(),
		}));

		let event = actions
			.iter()
			.find(|a| matches!(a, DiscoveryAction::Event(Event::Connected { .. })));
		assert!(event.is_some(), "expected Connected event for topic-subscribed peer");
		if let DiscoveryAction::Event(Event::Connected { id: eid, peer }) = event.unwrap() {
			assert_eq!(*eid, id);
			assert_eq!(*peer, remote);
		}
	}

	#[test]
	fn test_gossip_peer_unsubscribed_emits_disconnected_for_topic() {
		let local = test_peer(0);
		let remote = test_peer(1);
		let mut state = new_state(local);

		let (id, _) = connect(&mut state, vec![Discovery::Topic("my-topic".into())]);

		let topic = gossipsub::IdentTopic::new("my-topic");
		// first connect
		state.reduce(DiscoveryAction::GossipPeerSubscribed(GossipPeerSubscribedAction {
			peer_id: remote,
			topic: topic.hash(),
		}));

		// then unsubscribe
		let actions = state.reduce(DiscoveryAction::GossipPeerUnsubscribed(GossipPeerUnsubscribedAction {
			peer_id: remote,
			topic: topic.hash(),
		}));

		let event = actions
			.iter()
			.find(|a| matches!(a, DiscoveryAction::Event(Event::Disconnected { .. })));
		assert!(event.is_some(), "expected Disconnected event for topic-unsubscribed peer");
		if let DiscoveryAction::Event(Event::Disconnected { id: eid, peer }) = event.unwrap() {
			assert_eq!(*eid, id);
			assert_eq!(*peer, remote);
		}
	}

	#[test]
	fn test_gossip_ignores_local_peer() {
		let local = test_peer(0);
		let mut state = new_state(local);

		connect(&mut state, vec![Discovery::Topic("my-topic".into())]);

		let topic = gossipsub::IdentTopic::new("my-topic");
		let actions = state.reduce(DiscoveryAction::GossipPeerSubscribed(GossipPeerSubscribedAction {
			peer_id: local,
			topic: topic.hash(),
		}));

		assert!(actions.is_empty(), "should ignore local peer gossip events");
	}

	#[test]
	fn test_mdns_discovered_emits_dial_for_known_peers() {
		let local = test_peer(0);
		let remote = test_peer(1);
		let mut state = new_state(local);

		connect(&mut state, vec![peer_discovery(remote, vec![])]);

		let mut peers = BTreeSet::new();
		peers.insert(remote);
		let actions = state.reduce(DiscoveryAction::MdnsDiscovered(MdnsDiscoveredAction { peers }));

		let dial = actions.iter().find(|a| matches!(a, DiscoveryAction::DialPeer(_)));
		assert!(dial.is_some(), "expected DialPeer for mDNS-discovered peer");
	}
}
