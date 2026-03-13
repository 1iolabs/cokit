// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{
	bitswap::GetNetworkTask,
	didcomm::EncodedMessage,
	services::{
		connections::ConnectionMessage,
		discovery::DiscoveryApi,
		heads::HeadsApi,
		network::{
			CoNetworkTaskSpawner, DialNetworkTask, DidCommReceiveNetworkTask, DidCommSendNetworkTask,
			ListnersNetworkTask, NetworkMessage, PeersNetworkTask, SubscribeGossipTask,
		},
	},
};
use cid::Cid;
use co_actor::ActorHandle;
use co_identity::{Message, PrivateIdentity, PrivateIdentityBox};
use co_primitives::{Did, NetworkDidDiscovery};
use co_storage::StorageError;
use futures::{stream::BoxStream, StreamExt};
use libp2p_bitswap::Token;
use multiaddr::{Multiaddr, PeerId};
use std::{collections::BTreeSet, fmt::Debug, time::Duration};

#[derive(Debug, Clone)]
pub struct NetworkApi {
	pub(crate) _handle: ActorHandle<NetworkMessage>,
	pub(crate) connections: ActorHandle<ConnectionMessage>,
	pub(crate) discovery: DiscoveryApi,
	pub(crate) heads: HeadsApi,
	pub(crate) spawner: CoNetworkTaskSpawner,
}
impl NetworkApi {
	pub fn connections(&self) -> &ActorHandle<ConnectionMessage> {
		&self.connections
	}

	pub fn discovery(&self) -> &DiscoveryApi {
		&self.discovery
	}

	pub fn heads(&self) -> &HeadsApi {
		&self.heads
	}

	pub fn spawner(&self) -> &CoNetworkTaskSpawner {
		&self.spawner
	}

	/// Get our local peer id.
	pub fn local_peer_id(&self) -> PeerId {
		self.spawner.local_peer_id()
	}

	/// Get active listener addresses.
	/// If no listener is present it will wait for the first to come available.
	pub async fn listeners(&self, local: bool, external: bool) -> Result<BTreeSet<Multiaddr>, anyhow::Error> {
		ListnersNetworkTask::listeners(&self.spawner, local, external).await
	}

	/// Dial and wait for connection to be made or fail.
	pub async fn dial(&self, peer_id: Option<PeerId>, address: Vec<Multiaddr>) -> Result<PeerId, anyhow::Error> {
		// TODO: add to gossipsub?
		DialNetworkTask::dial(&self.spawner, peer_id, address).await
	}

	/// Subscribe identity for contact discovery.
	pub fn didcontact_subscribe<P>(&self, identity: P, network: NetworkDidDiscovery) -> Result<(), anyhow::Error>
	where
		P: PrivateIdentity + Debug + Clone + Send + Sync + 'static,
	{
		self.discovery
			.did_subscribe(Some(PrivateIdentityBox::new(identity)), Some(network))
	}

	/// Unsubscribe identity from contact discovery.
	pub fn didcontact_unsubscribe(&self, identity: Did) -> Result<(), anyhow::Error> {
		self.discovery.did_unsubscribe(Some(identity))
	}

	/// Subscribe identity for contact discovery.
	pub fn didcontact_subscribe_default(&self) -> Result<(), anyhow::Error> {
		self.discovery.did_subscribe(None, None)
	}

	/// Unsubscribe identity from contact discovery.
	pub fn didcontact_unsubscribe_default(&self) -> Result<(), anyhow::Error> {
		self.discovery.did_unsubscribe(None)
	}

	/// Send a DIDComm message to peers.
	/// Resolves as soon the message could be sent to one of the specified peers.
	pub async fn didcomm_send(
		&self,
		peers: impl IntoIterator<Item = PeerId>,
		message: EncodedMessage,
		timeout: Duration,
	) -> Result<PeerId, anyhow::Error> {
		DidCommSendNetworkTask::send(self.spawner.clone(), peers, message, timeout).await
	}

	/// Receive DIDComm messages.
	pub fn didcomm_receive(&self) -> BoxStream<'static, (PeerId, Message)> {
		DidCommReceiveNetworkTask::receive(self.spawner.clone()).boxed()
	}

	/// Open a stream that emit a item whenever the network conditions change.
	/// This can be used as a trigger for retries.
	pub fn network_changed(&self) -> BoxStream<'static, ()> {
		PeersNetworkTask::peers(&self.spawner).map(|_| ()).boxed()
	}

	/// Subscribe to a gossipsub topic by name.
	pub async fn subscribe_gossip_topic(&self, topic: &str) -> Result<bool, anyhow::Error> {
		SubscribeGossipTask::subscribe(self.spawner.clone(), libp2p::gossipsub::IdentTopic::new(topic)).await
	}

	/// Get block `cid` from bitswap.
	pub async fn bitswap_get(&self, cid: Cid, tokens: Vec<Token>, peers: BTreeSet<PeerId>) -> Result<(), StorageError> {
		GetNetworkTask::get(&self.spawner, cid, tokens, peers).await
	}
}
