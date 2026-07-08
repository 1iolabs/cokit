// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{
	bitswap::GetNetworkTask,
	didcomm::EncodedMessage,
	services::{
		connections::{CoConnectionOverview, ConnectionMessage, ConnectionOverview, NetworkOverview},
		discovery::DiscoveryApi,
		dns::{DnsApi, DnsSource},
		heads::HeadsApi,
		network::{
			CoNetworkTaskSpawner, DialNetworkTask, DidCommReceiveNetworkTask, DidCommSendNetworkTask,
			ListnersNetworkTask, NetworkMessage, PeersNetworkTask, SubscribeGossipTask, SwarmState,
			SwarmStateWatchTask,
		},
	},
};
use cid::Cid;
use co_actor::ActorHandle;
use co_identity::{Message, PrivateIdentity, PrivateIdentityBox};
use co_primitives::{CoId, Did, NetworkDidDiscovery};
use co_storage::StorageError;
use futures::{
	future,
	stream::{self, BoxStream},
	StreamExt,
};
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
	pub(crate) dns: DnsApi,
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

	/// Get a read-only snapshot of the current network state for diagnostics.
	pub async fn overview(&self) -> Result<NetworkOverview, anyhow::Error> {
		let local_peer_id = self.local_peer_id();

		// listeners + mDNS in a single swarm read: the watch task's first item.
		let mut swarm_state = SwarmStateWatchTask::watch(&self.spawner);
		let (listeners, mdns) = swarm_state.next().await.unwrap_or_default();

		let connections = self.connections.request(ConnectionMessage::Overview).await?;
		let dns = self.dns.source().await?;

		Ok(NetworkOverview { local_peer_id, listeners, mdns, connections, dns })
	}

	/// Subscribe to a live stream of [`NetworkOverview`]s for diagnostics.
	pub fn overview_stream(&self) -> BoxStream<'static, NetworkOverview> {
		let local_peer_id = self.local_peer_id();
		let connections = self.connections.stream_graceful(ConnectionMessage::OverviewStream);
		let swarm_state = SwarmStateWatchTask::watch(&self.spawner);
		let dns_source = self.dns.source_stream();

		enum Update {
			Connections(ConnectionOverview),
			Swarm(SwarmState),
			Dns(Option<DnsSource>),
		}
		#[derive(Default)]
		struct Latest {
			listeners: BTreeSet<Multiaddr>,
			mdns: BTreeSet<PeerId>,
			connections: ConnectionOverview,
			dns: Option<DnsSource>,
		}

		stream::select(
			stream::select(connections.map(Update::Connections), swarm_state.map(Update::Swarm)),
			dns_source.map(Update::Dns),
		)
		.scan(Latest::default(), move |latest, update| {
			match update {
				Update::Connections(connections) => latest.connections = connections,
				Update::Swarm((listeners, mdns)) => {
					latest.listeners = listeners;
					latest.mdns = mdns;
				},
				Update::Dns(dns) => latest.dns = dns,
			}
			future::ready(Some(NetworkOverview {
				local_peer_id,
				listeners: latest.listeners.clone(),
				mdns: latest.mdns.clone(),
				connections: latest.connections.clone(),
				dns: latest.dns,
			}))
		})
		.boxed()
	}

	/// Get a CO-scoped connection overview.
	/// Who we're connected to for `co` and how (per-endpoint transport/direction).
	pub async fn co_overview(&self, co: CoId) -> Result<CoConnectionOverview, anyhow::Error> {
		Ok(self
			.connections
			.request(move |response| ConnectionMessage::CoOverview(co, response))
			.await?)
	}

	/// Subscribe to a live CO-scoped connection overview, re-emitted on every connection-state change.
	pub fn co_overview_stream(&self, co: CoId) -> BoxStream<'static, CoConnectionOverview> {
		self.connections
			.stream_graceful(move |response| ConnectionMessage::CoOverviewStream(co, response))
			.boxed()
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

	/// Recover the network after a suspend/resume or interface change.
	///
	/// Asks the network actor to inject a recovery task that re-listens (fresh QUIC socket)
	/// and restarts mDNS. Returns once recovery has been initiated; healing happens
	/// asynchronously in the swarm loop. Idempotent and safe to call repeatedly.
	pub async fn recover(&self) -> Result<(), anyhow::Error> {
		self._handle.request(NetworkMessage::Recover).await?;
		Ok(())
	}
}
