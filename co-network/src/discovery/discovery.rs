use super::did_discovery::{DidDiscovery, DidDiscoveryMessage};
use crate::{
	didcomm,
	types::{
		layer_behaviour::LayerBehaviour,
		provider::{DidcommBehaviourProvider, GossipsubBehaviourProvider},
	},
};
use anyhow::anyhow;
use co_identity::{
	network_did_discovery, DidCommContext, DidCommHeader, DidCommPrivateContext, Identity, IdentityResolver,
	PrivateIdentity, PrivateIdentityBox,
};
use co_primitives::{Did, NetworkDidDiscovery, NetworkPeer, NetworkRendezvous};
use derive_more::From;
use futures::{future::BoxFuture, stream::FuturesUnordered, FutureExt, StreamExt};
use libp2p::{
	gossipsub::{self, TopicHash},
	mdns, rendezvous,
	swarm::{dial_opts::DialOpts, NetworkBehaviour, SwarmEvent},
	Multiaddr, PeerId, Swarm,
};
use std::{
	collections::{BTreeMap, BTreeSet, VecDeque},
	str::{from_utf8, FromStr},
	task::{Context, Poll},
	time::{Duration, Instant},
};

/// Single actionable discovery item with all context.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, From)]
pub enum Discovery {
	/// DID Discovery protocol.
	#[from]
	DidDiscovery(DidDiscovery),

	/// Discover subscribed peers from a topic.
	/// The value is the [`libp2p::gossipsub::TopicHash`] string representation.
	/// Note: This will not subscribe to an topic which needs to done by the caller.
	#[from]
	Topic(String),

	/// Rendezvouz protocol.
	#[from]
	Rendezvous(NetworkRendezvous),

	/// Direct peer connection.
	#[from]
	Peer(NetworkPeer),
}
impl Discovery {
	/// Create discover from peer.
	pub fn from_peer<'a>(peer: PeerId, addresses: impl IntoIterator<Item = &'a Multiaddr>) -> Self {
		Discovery::Peer(NetworkPeer {
			peer: peer.to_bytes(),
			addresses: addresses.into_iter().map(|i| i.to_string()).collect(),
		})
	}

	/// Validate the discovery contains parseable data.
	pub fn validate(&self) -> Result<(), anyhow::Error> {
		match self {
			Discovery::DidDiscovery(_item) => {
				// none?
			},
			Discovery::Topic(_item) => {
				// none?
			},
			Discovery::Rendezvous(item) => {
				for address in item.addresses.iter() {
					address.parse::<Multiaddr>()?;
				}
			},
			Discovery::Peer(item) => {
				PeerId::from_bytes(&item.peer)?;
			},
		}
		Ok(())
	}
}

/// Request to try to connect peers using suplied discovery methods.
struct DiscoveryConnectRequest {
	pub id: u64,
	/// The discovery items. Only contains validated ([`Discovery::validate`]) discovery items.
	pub discovery: BTreeSet<Discovery>,
	/// Cache for all direct PeerId we are intreseted in.
	pub discovery_peers: BTreeSet<PeerId>,
	pub start: Instant,
	pub timeout: Duration,
	pub max_peers: Option<u16>,
	pub connected_peers: BTreeSet<PeerId>,
	pub span: tracing::Span,
}
impl DiscoveryConnectRequest {
	/// Hit timeout using time?
	fn is_timedout(&self, time: Instant) -> bool {
		time - self.start > self.timeout
	}

	/// Hit max peers?
	fn is_max_peers(&self) -> bool {
		match (self.max_peers, self.connected_peers.len()) {
			(Some(max), len) if len >= max as usize => true,
			_ => false,
		}
	}

	fn build(&mut self) -> Result<(), anyhow::Error> {
		for item in &self.discovery {
			item.validate()?;
		}
		self.build_discovery_peers()?;
		Ok(())
	}

	/// Create discovery_peers from discovery.
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

/// Active subscription listening for DID Discovery requests.
struct DidDiscoverySubscription {
	network: NetworkDidDiscovery,
	identity: PrivateIdentityBox,
}

/// Event.
#[derive(Debug, Clone)]
pub enum Event {
	// Resolved { id: u64, request: Discovery, peers: BTreeMap<PeerId, Vec<Multiaddr>> },
	/// A peer to be discovered has connected.
	Connected { id: u64, peer: PeerId },

	/// A peer to be discovered has disconnected.
	Disconnected { id: u64, peer: PeerId },

	/// A discovery connect has timedout.
	/// TODO: Does it always mean it has failed?
	Timeout { id: u64 },
}

/// Discovery event.
/// This wrapps events intended for library users and events which involve the swarm.
/// The events are splitted to not need an mutable swarm handle just to receive events.
/// The caller is responsible to call on_discovery_event with produced events when appropiate.
#[derive(Debug, Clone)]
pub enum DiscoveryEvent {
	/// Event.
	GenerateEvent(Event),

	/// A resolve request to us via did discovery gossip.
	/// With an pre-computed DIDComm response.
	DidResolve { from_peer: PeerId, response: String },

	/// A discovery request.
	DidDiscovery { discovery: DidDiscovery },

	/// We received an (validated) DIDComm message.
	ReceivedDidComm { peer_id: PeerId, header: DidCommHeader },

	/// A peer has been discovered by the swarm and we may need to dail it.
	PeerDiscoverd { peer_id: PeerId },
}

/// Try to connect Peers using Discovery items.
///
/// Peer connections will be managed by the swarm (and its timeout).
pub struct DiscoveryState<R> {
	/// Our PeerId
	local_peer_id: PeerId,

	/// Next discovery request id.
	next_id: u64,

	/// Active discovery requests.
	requests: BTreeMap<u64, DiscoveryConnectRequest>,

	/// Active subscription listening for DID Discovery requests.
	did_subscriptions: BTreeMap<TopicHash, Vec<DidDiscoverySubscription>>,

	/// Pending events.
	events: VecDeque<DiscoveryEvent>,

	/// Pending DID Discovery requests. Insufficent peers.
	pending_discovery: VecDeque<(u64, TopicHash, DidDiscovery)>,

	/// Default discovery timeout.
	timeout: Duration,

	/// Default discovery max peers.
	max_peers: Option<u16>,

	// DID Identity resolver.
	resolver: R,

	// Pending events.
	future_events: FuturesUnordered<BoxFuture<'static, Option<DiscoveryEvent>>>,
}
impl<R> DiscoveryState<R>
where
	R: IdentityResolver + Clone + Send + Sync + 'static,
{
	pub fn new(resolver: R, local_peer_id: PeerId, timeout: Duration, max_peers: Option<u16>) -> Self {
		Self {
			local_peer_id,
			next_id: 1,
			requests: Default::default(),
			timeout,
			max_peers,
			events: Default::default(),
			did_subscriptions: Default::default(),
			pending_discovery: Default::default(),
			future_events: Default::default(),
			resolver,
		}
	}

	/// Subscribe identity for DID Discovery.
	pub fn did_discovery_subscribe<B, P>(
		&mut self,
		swarm: &mut Swarm<B>,
		network: Option<NetworkDidDiscovery>,
		identity: P,
	) -> Result<(), anyhow::Error>
	where
		B: DiscoveryBehaviour,
		P: PrivateIdentity + Send + Sync + 'static,
	{
		// network
		let network = network_did_discovery(&identity, network)?;

		// topic
		let topic = did_discovery_topic(&network);

		// add
		self.did_subscriptions
			.entry(topic.hash())
			.or_insert(Default::default())
			.push(DidDiscoverySubscription { identity: PrivateIdentityBox::new(identity), network: network.clone() });

		// subscribe
		let subscriptions_count = self.did_subscriptions.get(&topic.hash()).map(|v| v.len()).unwrap_or(0);
		if subscriptions_count == 1 {
			did_discovery_subscribe(swarm, &network)?;
		}

		// result
		Ok(())
	}

	/// Unsubscribe identity for DID Discovery.
	pub fn did_discovery_unsubscribe<B: DiscoveryBehaviour>(
		&mut self,
		swarm: &mut Swarm<B>,
		did: &Did,
	) -> Result<(), anyhow::Error> {
		// remove one subscription
		let mut remove_topic = None;
		let mut remove_subscription = None;
		for (topic, subscriptions) in self.did_subscriptions.iter_mut() {
			for (index, subscription) in subscriptions.iter().enumerate() {
				if subscription.identity.identity() == did {
					let remove = subscriptions.remove(index);
					if subscriptions.is_empty() {
						remove_topic = Some(topic.to_owned());
						remove_subscription = Some(remove.network);
					}
					break;
				}
			}
		}

		// remove
		if let Some(remove_topic) = remove_topic {
			self.did_subscriptions.remove(&remove_topic);
		}

		// unsubscribe
		if let Some(remove_subscription) = remove_subscription {
			did_discovery_unsubscribe(swarm, &remove_subscription)?;
		}

		// result
		Ok(())
	}

	/// Connect peers.
	pub fn connect<B>(
		&mut self,
		swarm: &mut Swarm<B>,
		discovery: impl IntoIterator<Item = Discovery>,
	) -> Result<u64, ConnectError>
	where
		B: DiscoveryBehaviour,
	{
		// id
		let id = self.next_id;
		self.next_id += 1;

		// tracing
		let span = tracing::trace_span!("discovery", discovery_id = id);
		let _enter = span.enter();

		// request
		let mut request = DiscoveryConnectRequest {
			id,
			discovery: discovery.into_iter().collect(),
			start: Instant::now(),
			max_peers: self.max_peers,
			timeout: self.timeout,
			discovery_peers: Default::default(),
			connected_peers: Default::default(),
			span: span.clone(),
		};
		request.build()?;

		// log
		tracing::trace!(timeout = ?request.timeout, discovery = ?request.discovery, "discovery");

		// add
		self.requests.insert(id, request);

		// connect
		match self.try_connect(swarm, id) {
			Ok(_) => Ok(id),
			Err(err) => {
				tracing::trace!(?err, "discovery-failure");
				self.release(id);
				Err(err)
			},
		}
	}

	fn try_connect<B>(&mut self, swarm: &mut Swarm<B>, request_id: u64) -> Result<(), ConnectError>
	where
		B: DiscoveryBehaviour,
	{
		let request = self.requests.get_mut(&request_id).ok_or(ConnectError::InvalidArgument)?;

		// connect
		let mut discovery_used = 0;
		for item in request.discovery.clone().into_iter() {
			match item {
				Discovery::DidDiscovery(item) => {
					let topic = did_discovery_topic(&item.network);
					let topic_hash = topic.hash();

					// we only use did discovery if the DID is currently subscribed.
					// this is because gossipsub only can publish messages when subscribed
					// todo: really?
					// note: currently we can only receive requests for DID which we also subscribed to
					//       so when we may change this we need to keep track of connection requests for
					//       dids/identities.
					if self.did_subscriptions.get(&topic.hash()).is_none() {
						tracing::trace!(network = ?item.network, ?topic, "discovery-did-unsubscribed");
						continue;
					}
					// let identity = subscriptions.iter().find(|subscription| {
					// 	println!("id: {}", subscription.identity.identity());
					// 	println!("item: {}", item.did);
					//  // todo: we need to amtch from no to
					// 	subscription.identity.identity() == item.did
					// });
					// if identity.is_none() {
					// 	continue;
					// }

					// publish
					match did_discovery(swarm, &item) {
						Ok(_) => {
							tracing::trace!(network = ?item.network, ?topic, "discovery-did-published");
						},

						// we try again when a peer subscribes
						Err(gossipsub::PublishError::InsufficientPeers) => {
							tracing::trace!(network = ?item.network, ?topic, "discovery-did-pending-insufficient-peers");
							self.pending_discovery.push_back((request.id, topic_hash, item.clone()));
						},

						// forward other errors
						Err(err) => {
							tracing::trace!(?err, network = ?item.network, ?topic, "discovery-did-failed");
							return Err(ConnectError::Other(err.into()));
						},
					};
				},
				Discovery::Topic(item) => {
					let hash = libp2p::gossipsub::TopicHash::from_raw(item);
					for peer in swarm.behaviour().gossipsub().mesh_peers(&hash) {
						peer_connected(request, &mut self.events, *peer);
					}
				},
				Discovery::Rendezvous(_item) => {
					// TODO: implement
					continue;
				},
				Discovery::Peer(item) => {
					peer(swarm, request, &mut self.events, peer_to_dial_opts(&item)?)?;
				},
			}
			discovery_used += 1;
		}

		// result
		if discovery_used == 0 {
			return Err(ConnectError::NoNetwork);
		}
		Ok(())
	}

	/// Returns currently connected peers for an request id. If request can not be found return an emty set.
	pub fn peers(&self, id: u64) -> BTreeSet<PeerId> {
		self.requests.get(&id).map(|r| &r.connected_peers).cloned().unwrap_or_default()
	}

	/// Release (may disconnect) discovered peers.
	pub fn release(&mut self, id: u64) {
		tracing::trace!(parent: self.requests.get(&id).and_then(|s| s.span.id()), "discovery-release");
		self.pending_discovery.retain(|(request, _, _)| *request != id);
		self.requests.remove(&id);
	}

	/// Timout requests and release them.
	fn timeout(&mut self) {
		let time = Instant::now();
		let timeout: BTreeSet<u64> = self
			.requests
			.iter()
			.filter(|(_, request)| request.is_timedout(time))
			.map(|(request_id, _)| *request_id)
			.collect();
		for request_id in timeout.into_iter() {
			self.release(request_id);
			self.events
				.push_back(DiscoveryEvent::GenerateEvent(Event::Timeout { id: request_id }));
		}
	}

	/// Iterate over all direct peer connection networks.
	fn all_discovery_peers(&self) -> impl Iterator<Item = (&DiscoveryConnectRequest, PeerId)> {
		self.requests.iter().flat_map(|(_, discovery_request)| {
			discovery_request.discovery_peers.iter().map(move |p| (discovery_request, *p))
		})
	}

	/// Iterate over all direct peer connection networks.
	fn all_discovery_diddiscovery(&self) -> impl Iterator<Item = (&DiscoveryConnectRequest, &DidDiscovery)> {
		self.requests.iter().flat_map(|(_, discovery_request)| {
			discovery_request.discovery.iter().filter_map(move |discovery| match discovery {
				Discovery::DidDiscovery(network) => Some((discovery_request, network)),
				_ => None,
			})
		})
	}

	/// Iterate over all direct peer connection networks.
	fn all_discovery_topics(&self) -> impl Iterator<Item = (&DiscoveryConnectRequest, &str)> {
		self.requests.iter().flat_map(|(_, discovery_request)| {
			discovery_request.discovery.iter().filter_map(move |discovery| match discovery {
				Discovery::Topic(topic) => Some((discovery_request, topic.as_str())),
				_ => None,
			})
		})
	}

	/// Handle GossipSub events.
	///
	/// Specifically:
	/// - Receiving DID Discovery messages addressed to us.
	/// - Handle Topic subscriptions.
	fn on_gossip_event(&mut self, event: &gossipsub::Event) {
		match event {
			gossipsub::Event::Message { propagation_source: _, message_id: _, message } => {
				// did discovery topic?
				if let Some(request_from_peer) = message.source {
					let subscriptions: Option<&Vec<DidDiscoverySubscription>> =
						self.did_subscriptions.get(&message.topic);
					if let Some(subscriptions) = subscriptions {
						// parse string
						let data = match from_utf8(&message.data) {
							Ok(s) => s,
							Err(_err) => {
								#[cfg(debug_assertions)]
								tracing::debug!(err = ?_err, "recevice-invalid-message");
								return;
							},
						};

						// receive
						self.future_events.push(
							did_discovery_receive(
								data.to_owned(),
								request_from_peer,
								self.resolver.clone(),
								subscriptions
									.iter()
									.map(|s| s.identity.didcomm_private())
									.filter_map(|item| item)
									.collect(),
							)
							.boxed(),
						);
					}
				}
			},
			gossipsub::Event::Subscribed { peer_id, topic } => {
				// filter out self subscribe
				if peer_id == &self.local_peer_id {
					return;
				}

				// DidDiscovery: move pending discoveries to events when its topic has subscribed
				loop {
					if let Some((index, _)) = self
						.pending_discovery
						.iter()
						.enumerate()
						.find(|(_, (_, pending_topic, _))| pending_topic == topic)
					{
						if let Some((_request, _topic, discovery)) = self.pending_discovery.remove(index) {
							self.events.push_back(DiscoveryEvent::DidDiscovery { discovery });
						}
					} else {
						break;
					}
				}

				// Topic: dispatch connected events for new mesh subscribription peers
				for request_id in self
					.all_discovery_topics()
					.filter(|(_, request_topic)| topic.as_str() == *request_topic)
					.map(|(request, _)| request.id)
					.collect::<Vec<_>>()
				{
					if let Some(request) = self.requests.get_mut(&request_id) {
						peer_connected(request, &mut self.events, *peer_id);
					}
				}
			},
			gossipsub::Event::Unsubscribed { peer_id, topic } => {
				// filter out self subscribe
				if peer_id == &self.local_peer_id {
					return;
				}

				// Topic: dispatch disconnected events for mesh unsubscribed peers
				for request_id in self
					.all_discovery_topics()
					.filter(|(_, request_topic)| topic.as_str() == *request_topic)
					.map(|(request, _)| request.id)
					.collect::<Vec<_>>()
				{
					if let Some(request) = self.requests.get_mut(&request_id) {
						peer_disconnected(request, &mut self.events, *peer_id);
					}
				}
			},
			_ => {},
		}
	}

	/// Handle DIDComm events.
	///
	/// Specifically:
	/// - Handle responses (type=diddiscovery) to DID Discovery messages.
	///
	/// TODO: Validate did message sender / recevier?
	fn on_didcomm_event(&mut self, event: &didcomm::Event) {
		match event {
			didcomm::Event::Received { peer_id, message } => {
				let message_type: Option<DidDiscoveryMessage> =
					DidDiscoveryMessage::try_from(message.header().message_type.clone()).ok();
				if message_type == Some(DidDiscoveryMessage::Resolve) {
					self.events.push_back(DiscoveryEvent::ReceivedDidComm {
						peer_id: peer_id.clone(),
						header: message.header().to_owned(),
					})
				}
			},
			_ => {},
			// didcomm::Event::Sent { peer_id, message } => todo!(),
			// didcomm::Event::OutboundFailure { peer_id, error, message } => todo!(),
		}
	}

	/// Handle mDNS events.
	///
	/// Specifically:
	/// - Dail peers which we want to discover.
	fn on_mdns_event(&mut self, event: &mdns::Event) {
		match event {
			mdns::Event::Discovered(items) => {
				let discovered_peers: BTreeSet<&PeerId> = items.iter().map(|(peer, _)| peer).collect();
				self.events.extend(
					self.all_discovery_peers()
						// skip connected peers
						.filter(|(request, peer)| !request.connected_peers.contains(peer))
						// max peers
						.filter(|(request, _)| !request.is_max_peers())
						// discovered peers
						.filter(|(_, peer)| discovered_peers.contains(peer))
						.map(|(_, peer_id)| DiscoveryEvent::PeerDiscoverd { peer_id })
						.collect::<Vec<_>>(),
				);
			},
			mdns::Event::Expired(_) => {},
		}
	}

	fn on_rendezvous_client_event(&mut self, _event: &rendezvous::client::Event) {
		// TODO: implement
	}
}
impl<B, R> LayerBehaviour<B> for DiscoveryState<R>
where
	B: DiscoveryBehaviour,
	R: IdentityResolver + Clone + Send + Sync + 'static,
{
	type ToSwarm = Event;
	type ToLayer = DiscoveryEvent;

	/// Handle swarm events.
	///
	/// Specifically:
	/// - Emit events for direct peer connections.
	/// - Forward behaviour events to handlers.
	fn on_swarm_event(&mut self, event: &SwarmEvent<<B as NetworkBehaviour>::ToSwarm>) {
		match event {
			SwarmEvent::ConnectionEstablished {
				peer_id,
				connection_id: _,
				endpoint: _,
				num_established,
				concurrent_dial_errors: _,
				established_in: _,
			} => {
				// only for the first connection
				if num_established.get() == 1 {
					// find all requests looking for this peer id
					let requests: BTreeSet<u64> = self
						.all_discovery_peers()
						.filter(|(_, request_peer)| peer_id == request_peer)
						.map(|(request, _)| request.id)
						.collect();
					for request_id in requests.into_iter() {
						if let Some(request) = self.requests.get_mut(&request_id) {
							peer_connected(request, &mut self.events, *peer_id);
						}
					}
				}
			},
			SwarmEvent::ConnectionClosed { peer_id, connection_id: _, endpoint: _, num_established, cause: _ } => {
				// only for the last connection
				if *num_established == 0 {
					// find all requests looking for this peer id
					let requests: BTreeSet<u64> = self
						.all_discovery_peers()
						.filter(|(_, request_peer)| peer_id == request_peer)
						.map(|(request, _)| request.id)
						.collect();
					for request_id in requests.into_iter() {
						if let Some(request) = self.requests.get_mut(&request_id) {
							peer_disconnected(request, &mut self.events, *peer_id);
						}
					}
				}
			},
			SwarmEvent::Behaviour(behaviour_event) => {
				if let Some(gossip_event) = B::gossipsub_event(behaviour_event) {
					self.on_gossip_event(gossip_event);
				}
				if let Some(didcomm_event) = B::didcomm_event(behaviour_event) {
					self.on_didcomm_event(didcomm_event);
				}
				if let Some(mdns_event) = B::mdns_event(behaviour_event) {
					self.on_mdns_event(mdns_event);
				}
				if let Some(rendezvous_client_event) = B::rendezvous_client_event(behaviour_event) {
					self.on_rendezvous_client_event(rendezvous_client_event);
				}
			},
			_ => {},
		}
	}

	/// Handle discovery event which involves the swarm.
	/// Other events are returned.
	fn on_layer_event(&mut self, swarm: &mut Swarm<B>, event: Self::ToLayer) -> Option<Self::ToSwarm> {
		match event {
			DiscoveryEvent::DidResolve { from_peer, response } => {
				swarm.behaviour_mut().didcomm_mut().send(&from_peer, response.into());
				None
			},
			DiscoveryEvent::DidDiscovery { discovery } => {
				match did_discovery(swarm, &discovery) {
					Ok(_) => {},
					Err(err) => {
						tracing::warn!(?discovery, ?err, "did_discovery-publish-failed")
					},
				};
				None
			},
			DiscoveryEvent::ReceivedDidComm { peer_id, header } => {
				let message_type: Option<DidDiscoveryMessage> = DidDiscoveryMessage::from_str(&header.message_type);
				if message_type == Some(DidDiscoveryMessage::Resolve) {
					for request_id in self
						.all_discovery_diddiscovery()
						.filter(|(_, discovery)| header.thid.as_ref() == Some(&discovery.message_id))
						.map(|(request, _)| request.id)
						.collect::<Vec<_>>()
					{
						if let Some(request) = self.requests.get_mut(&request_id) {
							if !request.connected_peers.contains(&peer_id) {
								request.connected_peers.insert(peer_id);
								self.events.push_back(DiscoveryEvent::GenerateEvent(Event::Connected {
									id: request_id,
									peer: peer_id,
								}));
							}
						}
					}
				}
				None
			},
			DiscoveryEvent::PeerDiscoverd { peer_id } => {
				// we discoverd a peer we intreset in though an request
				// so we jsut dail it. once the connection is made we will be notified by
				// [`SwarmEvent::ConnectionEstablished`].
				if !swarm.is_connected(&peer_id) {
					let opts = DialOpts::peer_id(peer_id).build();
					match swarm.dial(opts) {
						Ok(_) => {},
						Err(err) => {
							tracing::warn!(?err, ?peer_id, "discovery-peer-dail-failed");
						},
					}
				}
				None
			},
			DiscoveryEvent::GenerateEvent(event) => Some(event),
		}
	}

	fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Self::ToLayer> {
		// timeouts
		self.timeout();

		// events
		if let Some(event) = self.events.pop_front() {
			return Poll::Ready(event);
		}

		// pending futures
		match self.future_events.poll_next_unpin(cx) {
			Poll::Ready(Some(Some(event))) => return Poll::Ready(event),
			_ => {},
		}

		// pending
		Poll::Pending
	}
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
	#[error("No network is usable to connect")]
	NoNetwork,

	#[error("Invalid argument")]
	InvalidArgument,

	#[error("Connection error")]
	Other(#[from] anyhow::Error),
}

pub trait DiscoveryBehaviour: NetworkBehaviour + GossipsubBehaviourProvider + DidcommBehaviourProvider {
	fn rendezvous_client_mut(&mut self) -> Option<&mut rendezvous::client::Behaviour>;
	fn rendezvous_client_event(event: &<Self as NetworkBehaviour>::ToSwarm) -> Option<&rendezvous::client::Event>;

	fn mdns_mut(&mut self) -> Option<&mut mdns::tokio::Behaviour>;
	fn mdns_event(event: &<Self as NetworkBehaviour>::ToSwarm) -> Option<&mdns::Event>;

	// fn kad_mut(&mut self) -> Option<&mut kad::Behaviour<kad::store::MemoryStore>>;
	// fn kad_mut(event: &<Self as NetworkBehaviour>::ToSwarm) -> Option<&kad::Event>;
}

/// Acccept DidDiscoveryMessage::Discover events and respond with DidDiscoveryMessage::Resolve.
async fn did_discovery_receive<R: IdentityResolver>(
	data: String,
	request_from_peer: PeerId,
	resolver: R,
	contexts: Vec<DidCommPrivateContext>,
) -> Option<DiscoveryEvent> {
	let result = didcomm_receive(&data, resolver, contexts.into_iter()).await;
	if let Some((request_header, _, didcomm_private)) = result {
		if DidDiscoveryMessage::from_str(&request_header.message_type) == Some(DidDiscoveryMessage::Discover) {
			match did_discovery_resolve(&didcomm_private, request_from_peer, request_header) {
				Ok(event) => return Some(event),
				Err(err) => {
					tracing::warn!(?err, "discovery-did-resolve-failed");
				},
			}
		}
	}
	None
}

/// Respond to an did discovery resolve request.
fn did_discovery_resolve(
	identity: &DidCommPrivateContext,
	request_from_peer: PeerId,
	request: DidCommHeader,
) -> Result<DiscoveryEvent, anyhow::Error> {
	let request_from = request.from.ok_or(anyhow!("Missing from header field"))?;

	// response
	let mut response = DidCommHeader::new(DidDiscoveryMessage::Resolve.to_string());
	response.thid = Some(request.id);
	response.from = Some(identity.did().to_owned());
	response.to.insert(request_from.clone());

	// message
	let message = identity.jws(response, "null")?;

	// result
	Ok(DiscoveryEvent::DidResolve { from_peer: request_from_peer, response: message })
}

/// Try to receive a message (data) with one of the supplied identities.
async fn didcomm_receive<R: IdentityResolver>(
	data: &str,
	resolver: R,
	contexts: impl Iterator<Item = DidCommPrivateContext>,
) -> Option<(DidCommHeader, String, DidCommPrivateContext)> {
	for didcomm_private in contexts {
		match didcomm_private.receive(&resolver, data).await {
			Ok((header, body)) => return Some((header, body, didcomm_private)),
			Err(_err) => {
				// note: this will happen on purpose because we check the message against all identities and only
				// one will match.
				#[cfg(debug_assertions)]
				tracing::debug!(err = ?_err, ?data, "jwe-receive-failed");
			},
		}
	}
	None
}

/// Get did discovery gossipsub topic.
fn did_discovery_topic(network: &NetworkDidDiscovery) -> gossipsub::IdentTopic {
	gossipsub::IdentTopic::new(did_discovery_topic_str(network))
}

/// Get did discovery gossipsub topic as string.
fn did_discovery_topic_str(network: &NetworkDidDiscovery) -> &str {
	network.topic.as_deref().unwrap_or("co-contact")
}

/// Subscribe did discovery gossipsub topic.
fn did_discovery_subscribe<B: DiscoveryBehaviour>(
	swarm: &mut Swarm<B>,
	did_discovery: &NetworkDidDiscovery,
) -> Result<bool, gossipsub::SubscriptionError> {
	Ok(swarm
		.behaviour_mut()
		.gossipsub_mut()
		.subscribe(&did_discovery_topic(did_discovery))?)
}

/// Unsubscribe did discovery gossipsub topic.
fn did_discovery_unsubscribe<B: DiscoveryBehaviour>(
	swarm: &mut Swarm<B>,
	did_discovery: &NetworkDidDiscovery,
) -> Result<bool, gossipsub::PublishError> {
	Ok(swarm
		.behaviour_mut()
		.gossipsub_mut()
		.unsubscribe(&did_discovery_topic(did_discovery))?)
}

/// Publish to did discovery gossipsub topic.
fn did_discovery<B: DiscoveryBehaviour>(
	swarm: &mut Swarm<B>,
	discovery: &DidDiscovery,
) -> Result<gossipsub::MessageId, gossipsub::PublishError> {
	swarm
		.behaviour_mut()
		.gossipsub_mut()
		.publish(did_discovery_topic(&discovery.network), discovery.message.clone())
}

/// Try to dail peer.
/// Note: mDNS will automatically help to dail the PeerId using `handle_pending_outbound_connection`.
fn peer<B: DiscoveryBehaviour>(
	swarm: &mut Swarm<B>,
	request: &mut DiscoveryConnectRequest,
	events: &mut VecDeque<DiscoveryEvent>,
	opts: DialOpts,
) -> Result<(), anyhow::Error> {
	// self?
	if opts.get_peer_id().as_ref() == Some(swarm.local_peer_id()) {
		return Ok(());
	}

	// already connected?
	if let Some(peer) = opts.get_peer_id() {
		if swarm.is_connected(&peer) {
			peer_connected(request, events, peer);
			return Ok(());
		}
	}

	// dail
	swarm.dial(opts)?;

	// done
	Ok(())
}

fn peer_to_dial_opts(item: &NetworkPeer) -> Result<DialOpts, anyhow::Error> {
	let peer: PeerId = PeerId::from_bytes(&item.peer)?;
	let addresses = item
		.addresses
		.iter()
		.map(|address| Multiaddr::from_str(&address))
		.collect::<Result<BTreeSet<_>, _>>()?;
	Ok(DialOpts::peer_id(peer)
		.addresses(addresses.clone().into_iter().collect())
		.build())
}

fn peer_connected(request: &mut DiscoveryConnectRequest, events: &mut VecDeque<DiscoveryEvent>, peer: PeerId) {
	if request.connected_peers.insert(peer) {
		tracing::trace!(parent: request.span.id(), ?peer, "discovery-connected");
		events.push_back(DiscoveryEvent::GenerateEvent(Event::Connected { id: request.id, peer }));
	}
}

fn peer_disconnected(request: &mut DiscoveryConnectRequest, events: &mut VecDeque<DiscoveryEvent>, peer: PeerId) {
	if request.connected_peers.remove(&peer) {
		tracing::trace!(parent: request.span.id(), ?peer, "discovery-disconnected");
		events.push_back(DiscoveryEvent::GenerateEvent(Event::Disconnected { id: request.id, peer }));
	}
}

#[cfg(test)]
mod tests {
	use super::DiscoveryBehaviour;
	use crate::{
		didcomm,
		discovery::{DidDiscovery, DidDiscoveryMessage, Discovery, DiscoveryState, Event},
		types::{
			layer_behaviour::{Layer, LayerBehaviour},
			provider::{DidcommBehaviourProvider, GossipsubBehaviourProvider},
		},
	};
	use co_identity::{
		DidKeyIdentity, DidKeyIdentityResolver, IdentityResolver, MemoryPrivateIdentityResolver, PrivateIdentity,
		PrivateIdentityBox, PrivateIdentityResolver,
	};
	use futures::{select, FutureExt, StreamExt};
	use libp2p::{
		gossipsub,
		identity::Keypair,
		noise, rendezvous,
		swarm::{dial_opts::DialOpts, NetworkBehaviour},
		tcp, yamux, Multiaddr, PeerId, Swarm, SwarmBuilder,
	};
	use std::{time::Duration, vec};

	#[derive(NetworkBehaviour)]
	struct TestBehaviour {
		didcomm: didcomm::Behaviour,
		gossipsub: gossipsub::Behaviour,
	}
	impl TestBehaviour {
		pub fn new(keypair: Keypair, identities: Vec<PrivateIdentityBox>) -> Self {
			let gossipsub_config = gossipsub::ConfigBuilder::default()
				.max_transmit_size(256 * 1024)
				.build()
				.expect("valid config");
			let gossipsub_behaviour =
				gossipsub::Behaviour::new(gossipsub::MessageAuthenticity::Signed(keypair), gossipsub_config)
					.expect("gossipsub");
			let didcomm_behaviour = didcomm::Behaviour::new(
				DidKeyIdentityResolver::new().boxed(),
				MemoryPrivateIdentityResolver::from(identities).boxed(),
				didcomm::Config { auto_dail: false },
			);
			TestBehaviour { didcomm: didcomm_behaviour, gossipsub: gossipsub_behaviour }
		}
	}
	impl DidcommBehaviourProvider for TestBehaviour {
		fn didcomm(&self) -> &didcomm::Behaviour {
			&self.didcomm
		}

		fn didcomm_mut(&mut self) -> &mut didcomm::Behaviour {
			&mut self.didcomm
		}

		fn didcomm_event(event: &<Self as NetworkBehaviour>::ToSwarm) -> Option<&didcomm::Event> {
			match event {
				TestBehaviourEvent::Didcomm(e) => Some(e),
				_ => None,
			}
		}

		fn into_didcomm_event(
			event: <Self as NetworkBehaviour>::ToSwarm,
		) -> Result<didcomm::Event, <Self as NetworkBehaviour>::ToSwarm> {
			match event {
				TestBehaviourEvent::Didcomm(e) => Ok(e),
				e => Err(e),
			}
		}
	}
	impl GossipsubBehaviourProvider for TestBehaviour {
		fn gossipsub(&self) -> &gossipsub::Behaviour {
			&self.gossipsub
		}

		fn gossipsub_mut(&mut self) -> &mut gossipsub::Behaviour {
			&mut self.gossipsub
		}

		fn gossipsub_event(event: &<Self as NetworkBehaviour>::ToSwarm) -> Option<&gossipsub::Event> {
			match event {
				TestBehaviourEvent::Gossipsub(e) => Some(e),
				_ => None,
			}
		}

		fn into_gossipsub_event(
			event: <Self as NetworkBehaviour>::ToSwarm,
		) -> Result<gossipsub::Event, <Self as NetworkBehaviour>::ToSwarm> {
			match event {
				TestBehaviourEvent::Gossipsub(e) => Ok(e),
				e => Err(e),
			}
		}
	}
	impl DiscoveryBehaviour for TestBehaviour {
		fn rendezvous_client_mut(&mut self) -> Option<&mut rendezvous::client::Behaviour> {
			None
		}

		fn rendezvous_client_event(_event: &<Self as NetworkBehaviour>::ToSwarm) -> Option<&rendezvous::client::Event> {
			None
		}

		fn mdns_mut(&mut self) -> Option<&mut libp2p::mdns::tokio::Behaviour> {
			None
		}

		fn mdns_event(_event: &<Self as NetworkBehaviour>::ToSwarm) -> Option<&libp2p::mdns::Event> {
			None
		}
	}

	struct Peer {
		peer_id: PeerId,
		addr: Multiaddr,
		swarm: Swarm<TestBehaviour>,
	}
	impl Peer {
		fn new(identities: Vec<PrivateIdentityBox>) -> Self {
			let mut swarm = SwarmBuilder::with_new_identity()
				.with_tokio()
				.with_tcp(tcp::Config::default(), noise::Config::new, yamux::Config::default)
				.unwrap()
				.with_behaviour(|k| Ok(TestBehaviour::new(k.clone(), identities)))
				.unwrap()
				.with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(20)))
				.build();
			swarm.listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap()).unwrap();
			while swarm.next().now_or_never().is_some() {}
			let addr = Swarm::listeners(&swarm).next().unwrap().clone();
			Self { peer_id: swarm.local_peer_id().clone(), addr, swarm }
		}

		fn peer_id(&self) -> PeerId {
			self.peer_id
		}

		fn add_address(&mut self, peer: &Peer) {
			self.swarm
				.dial(
					DialOpts::peer_id(peer.peer_id.clone())
						.addresses(vec![peer.addr.clone()])
						.build(),
				)
				.unwrap();
			// when we dail just the peerid we always get an dail error because we have no addresses
			// self.swarm.behaviour_mut().add_explicit_peer(co, peer.peer_id.clone());
		}

		fn swarm(&mut self) -> &mut Swarm<TestBehaviour> {
			&mut self.swarm
		}
	}

	#[tokio::test]
	async fn test_peer_discovery() {
		// tracing_subscriber::fmt()
		// 	.with_env_filter(tracing_subscriber::EnvFilter::new(format!(
		// 		"{}=trace",
		// 		module_path!().split(":").next().expect("module path")
		// 	)))
		// 	.try_init()
		// 	.ok();

		// peers
		let mut peer1 = Peer::new(vec![]);
		let mut peer2 = Peer::new(vec![]);
		let peer1_id = peer1.peer_id();
		let peer2_id = peer2.peer_id();

		// states
		let mut discovery1 = Layer::new(
			peer1.swarm().behaviour(),
			DiscoveryState::new(DidKeyIdentityResolver::new(), peer1_id, Duration::from_secs(10), None),
		);
		let mut discovery2 = Layer::new(
			peer2.swarm().behaviour(),
			DiscoveryState::new(DidKeyIdentityResolver::new(), peer2_id, Duration::from_secs(10), None),
		);

		// peer2: connect
		discovery2
			.layer_mut()
			.connect(peer2.swarm(), vec![Discovery::from_peer(peer1.peer_id, std::iter::once(&peer1.addr))])
			.unwrap();

		// run
		loop {
			select! {
				event = peer1.swarm().next() => {
					tracing::info!(?event, "peer1");
					discovery1.on_swarm_event(&event.unwrap());
				},
				event = peer2.swarm().next() => {
					tracing::info!(?event, "peer2");
					discovery2.on_swarm_event(&event.unwrap());
				},
				event = discovery1.next() => {
					tracing::info!(?event, "discovery1");
					discovery1.on_layer_event(peer1.swarm(), event.unwrap());
				},
				event = discovery2.next() => {
					tracing::info!(?event, "discovery2");
					match discovery2.on_layer_event(peer2.swarm(), event.unwrap()) {
						Some(Event::Connected {id: _, peer}) => {
							assert_eq!(peer, peer1.peer_id());
							break;
						},
						Some(Event::Timeout { id: _ }) => {
							panic!("timeout");
						}
						_ => {},
					}
				},
			};
		}
	}

	//#[tracing_test::traced_test]
	//#[test_log::test(tokio::test)]
	#[tokio::test]
	async fn test_did_discovery() {
		// tracing_subscriber::fmt()
		// 	.with_env_filter(tracing_subscriber::EnvFilter::new(format!(
		// 		"{}=trace",
		// 		module_path!().split(":").next().expect("module path")
		// 	)))
		// 	.try_init()
		// 	.ok();

		// identities
		let did1 = DidKeyIdentity::generate(Some(&vec![1; 32]));
		let did2 = DidKeyIdentity::generate(Some(&vec![2; 32]));

		// peers
		let mut peer1 = Peer::new(vec![did1.clone().boxed()]);
		let mut peer2 = Peer::new(vec![did2.clone().boxed()]);
		peer2.add_address(&peer1);
		let peer1_id = peer1.peer_id();
		let peer2_id = peer2.peer_id();

		// states
		let mut discovery1 = Layer::new(
			peer1.swarm().behaviour(),
			DiscoveryState::new(DidKeyIdentityResolver::new(), peer1_id, Duration::from_secs(10), None),
		);
		let mut discovery2 = Layer::new(
			peer2.swarm().behaviour(),
			DiscoveryState::new(DidKeyIdentityResolver::new(), peer2_id, Duration::from_secs(10), None),
		);

		// peer1: subscribe
		discovery1
			.layer_mut()
			.did_discovery_subscribe(peer1.swarm(), None, did1.clone())
			.unwrap();

		// peer2: subscribe
		discovery2
			.layer_mut()
			.did_discovery_subscribe(peer2.swarm(), None, did2.clone())
			.unwrap();

		// // wait subscribed
		// let (subscribe1, subscribe2) = join!(peer1.next(), peer2.next());
		// match subscribe1 {
		// 	Some(TestBehaviourEvent::Gossipsub(gossipsub::Event::Subscribed { peer_id, topic }))
		// 		if topic == IdentTopic::new("co-contact").hash() && peer_id == peer2.peer_id => {},
		// 	event => panic!("unexpected event: {:?}", event),
		// }
		// match subscribe2 {
		// 	Some(TestBehaviourEvent::Gossipsub(gossipsub::Event::Subscribed { peer_id, topic }))
		// 		if topic == IdentTopic::new("co-contact").hash() && peer_id == peer1.peer_id => {},
		// 	event => panic!("unexpected event: {:?}", event),
		// }

		// peer2: connect
		discovery2
			.layer_mut()
			.connect(
				peer2.swarm(),
				vec![DidDiscovery::create(peer2_id, &did2, &did1, None, DidDiscoveryMessage::Discover.to_string())
					.unwrap()
					.into()],
			)
			.unwrap();

		// run
		loop {
			select! {
				event = peer1.swarm().next() => {
					tracing::info!(?event, "peer1");
					discovery1.on_swarm_event(&event.unwrap());
				},
				event = peer2.swarm().next() => {
					tracing::info!(?event, "peer2");
					discovery2.on_swarm_event(&event.unwrap());
				},
				event = discovery1.next() => {
					tracing::info!(?event, "discovery1");
					discovery1.on_layer_event(peer1.swarm(), event.unwrap());
				},
				event = discovery2.next() => {
					tracing::info!(?event, "discovery2");
					match discovery2.on_layer_event(peer2.swarm(), event.unwrap()) {
						Some(Event::Connected {id: _, peer}) => {
							assert_eq!(peer, peer1.peer_id());
							break;
						},
						Some(Event::Timeout { id: _ }) => {
							panic!("timeout");
						}
						_ => {},
					}
				},
			};
		}
	}
}
