use crate::{
	bitswap::GetNetworkTask,
	didcomm::EncodedMessage,
	services::{
		connections::ConnectionMessage,
		heads::HeadsApi,
		network::{
			CoNetworkTaskSpawner, DialNetworkTask, DidCommReceiveNetworkTask, DidCommSendNetworkTask,
			DidDiscoverySubscribe, DidDiscoveryUnsubscribe, ListnersNetworkTask, NetworkMessage, PeersNetworkTask,
		},
	},
	types::network_task::NetworkTaskSpawner,
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
	pub(crate) heads: HeadsApi,
	pub(crate) spawner: CoNetworkTaskSpawner,
}
impl NetworkApi {
	pub fn connections(&self) -> &ActorHandle<ConnectionMessage> {
		&self.connections
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
	pub async fn didcontact_subscribe<P>(&self, identity: P, network: NetworkDidDiscovery) -> Result<(), anyhow::Error>
	where
		P: PrivateIdentity + Debug + Clone + Send + Sync + 'static,
	{
		let (task, result) = DidDiscoverySubscribe::new(PrivateIdentityBox::new(identity), Some(network));
		self.spawner.spawn(task)?;
		result.await??;
		Ok(())
	}

	/// Unsubscribe identity from contact discovery.
	pub async fn didcontact_unsubscribe(&self, identity: Did) -> Result<(), anyhow::Error> {
		let (task, result) = DidDiscoveryUnsubscribe::new(identity);
		self.spawner.spawn(task)?;
		result.await??;
		Ok(())
	}

	/// Subscribe identity for contact discovery.
	pub async fn didcontact_subscribe_default(&self) -> Result<(), anyhow::Error> {
		let (task, result) = DidDiscoverySubscribe::default();
		self.spawner.spawn(task)?;
		result.await??;
		Ok(())
	}

	/// Unsubscribe identity from contact discovery.
	pub async fn didcontact_unsubscribe_default(&self) -> Result<(), anyhow::Error> {
		let (task, result) = DidDiscoveryUnsubscribe::default();
		self.spawner.spawn(task)?;
		result.await??;
		Ok(())
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

	/// Get block `cid` from bitswap.
	pub async fn bitswap_get(&self, cid: Cid, tokens: Vec<Token>, peers: BTreeSet<PeerId>) -> Result<(), StorageError> {
		GetNetworkTask::get(&self.spawner, cid, tokens, peers).await
	}
}
