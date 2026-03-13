// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use super::state::DidDiscoverySubscription;
use crate::services::discovery;
use co_identity::DidCommHeader;
use derive_more::From;
use libp2p::{gossipsub::TopicHash, Multiaddr, PeerId};
use std::collections::BTreeSet;

/// All actions that flow through the discovery service.
#[derive(Debug, Clone, From)]
pub enum DiscoveryAction {
	// Caller actions
	/// Create a new discovery connect request.
	Connect(ConnectAction),
	/// Release a discovery request.
	Release(ReleaseAction),
	/// Subscribe identity for DID discovery.
	DidSubscribe(DidSubscribeAction),
	/// Unsubscribe identity from DID discovery.
	DidUnsubscribe(DidUnsubscribeAction),

	// Swarm event actions (from DiscoveryNetworkTask)
	/// A peer connection was established (first connection).
	PeerConnected(PeerConnectedAction),
	/// A peer connection was closed (last connection).
	PeerDisconnected(PeerDisconnectedAction),
	/// A gossipsub message was received.
	GossipMessage(GossipMessageAction),
	/// A peer subscribed to a gossipsub topic.
	GossipPeerSubscribed(GossipPeerSubscribedAction),
	/// A peer unsubscribed from a gossipsub topic.
	GossipPeerUnsubscribed(GossipPeerUnsubscribedAction),
	/// A DIDComm resolve response was received.
	DidCommReceived(DidCommReceivedAction),
	/// Peers discovered via mDNS.
	MdnsDiscovered(MdnsDiscoveredAction),

	// Internal actions (reducer to epics)
	/// Subscribe to a gossipsub topic.
	GossipSubscribe(GossipSubscribeAction),
	/// Unsubscribe from a gossipsub topic.
	GossipUnsubscribe(GossipUnsubscribeAction),
	/// Publish a DID discovery message to gossipsub.
	DidPublish(DidPublishAction),
	/// Query mesh peers for a topic.
	QueryMeshPeers(QueryMeshPeersAction),
	/// Dial a peer.
	DialPeer(DialPeerAction),
	/// Send a DIDComm resolve response and dial the peer.
	SendResolve(SendResolveAction),

	// Internal actions (epics to reducer)
	/// DID publish failed because no peers subscribed to topic.
	DidPublishPending(DidPublishPendingAction),
	/// Mesh peers query result.
	MeshPeersResult(MeshPeersResultAction),
	/// Async DID discovery message decrypted, ready to resolve.
	DidDecrypted(DidDecryptedAction),
	/// A dial attempt failed.
	DialFailed(DialFailedAction),
	/// A discovery request timed out.
	Timeout(TimeoutAction),

	// Outbound events (reducer to response streams)
	/// Event to emit to callers.
	Event(discovery::Event),
}

#[derive(Debug, Clone)]
pub struct ConnectAction {
	pub id: u64,
	pub discovery: BTreeSet<discovery::Discovery>,
}

#[derive(Debug, Clone)]
pub struct ReleaseAction {
	pub id: u64,
}

#[derive(Debug, Clone)]
pub struct DidSubscribeAction {
	pub subscription: DidDiscoverySubscription,
	pub topic_str: String,
}

#[derive(Debug, Clone)]
pub enum DidUnsubscribeAction {
	Identity(co_primitives::Did),
	Default,
}

#[derive(Debug, Clone)]
pub struct PeerConnectedAction {
	pub peer_id: PeerId,
}

#[derive(Debug, Clone)]
pub struct PeerDisconnectedAction {
	pub peer_id: PeerId,
}

#[derive(Debug, Clone)]
pub struct GossipMessageAction {
	pub topic: TopicHash,
	pub source: Option<PeerId>,
	pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct GossipPeerSubscribedAction {
	pub peer_id: PeerId,
	pub topic: TopicHash,
}

#[derive(Debug, Clone)]
pub struct GossipPeerUnsubscribedAction {
	pub peer_id: PeerId,
	pub topic: TopicHash,
}

#[derive(Debug, Clone)]
pub struct DidCommReceivedAction {
	pub peer_id: PeerId,
	pub header: DidCommHeader,
}

#[derive(Debug, Clone)]
pub struct MdnsDiscoveredAction {
	pub peers: BTreeSet<PeerId>,
}

#[derive(Debug, Clone)]
pub struct GossipSubscribeAction {
	pub topic: String,
}

#[derive(Debug, Clone)]
pub struct GossipUnsubscribeAction {
	pub topic: String,
}

#[derive(Debug, Clone)]
pub struct DidPublishAction {
	pub request_id: u64,
	pub discovery: discovery::DidDiscovery,
}

#[derive(Debug, Clone)]
pub struct QueryMeshPeersAction {
	pub request_id: u64,
	pub topic: String,
}

#[derive(Debug, Clone)]
pub struct DialPeerAction {
	pub request_id: Option<u64>,
	pub peer_id: PeerId,
	pub addresses: Vec<Multiaddr>,
}

#[derive(Debug, Clone)]
pub struct SendResolveAction {
	pub from_peer: PeerId,
	pub from_endpoints: BTreeSet<Multiaddr>,
	pub response: String,
}

#[derive(Debug, Clone)]
pub struct DidPublishPendingAction {
	pub request_id: u64,
	pub topic: TopicHash,
	pub discovery: discovery::DidDiscovery,
}

#[derive(Debug, Clone)]
pub struct MeshPeersResultAction {
	pub request_id: u64,
	pub peers: Vec<PeerId>,
}

#[derive(Debug, Clone)]
pub struct DidDecryptedAction {
	pub from_peer: PeerId,
	pub from_endpoints: BTreeSet<Multiaddr>,
	pub response: String,
}

#[derive(Debug, Clone)]
pub struct DialFailedAction {
	pub request_id: Option<u64>,
	pub peer_id: PeerId,
}

#[derive(Debug, Clone)]
pub struct TimeoutAction {
	pub id: u64,
}
