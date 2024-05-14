use super::did_discovery::DidDiscovery;
use crate::didcomm;
use anyhow::anyhow;
use co_identity::{DidCommHeader, DidCommPrivateContext, PrivateIdentity, PrivateIdentityBox};
use co_primitives::{Did, NetworkDidDiscovery, NetworkPeer, NetworkRendezvous};
use derive_more::From;
use futures::{stream::FusedStream, Stream};
use libp2p::{
	gossipsub::{self, TopicHash},
	mdns, rendezvous,
	swarm::{dial_opts::DialOpts, NetworkBehaviour, SwarmEvent},
	Multiaddr, PeerId, Swarm,
};
use std::{
	collections::{BTreeMap, BTreeSet, VecDeque},
	pin::Pin,
	str::{from_utf8, FromStr},
	task::{Context, Poll},
	time::{Duration, Instant},
};

/// Single actionable discovery item with all context.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, From)]
pub enum Discovery {
	#[from]
	DidDiscovery(DidDiscovery),
	#[from]
	Rendezvous(NetworkRendezvous),
	#[from]
	Peer(NetworkPeer),
}
impl Discovery {
	/// Validate the discovery contains parseable data.
	pub fn validate(&self) -> Result<(), anyhow::Error> {
		match self {
			Discovery::DidDiscovery(_item) => {
				// none?
			},
			Discovery::Rendezvous(item) =>
				for address in item.addresses.iter() {
					address.parse::<Multiaddr>()?;
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
	pub start: Instant,
	pub timeout: Duration,
	pub max_peers: Option<u16>,
	pub connected_peers: BTreeSet<PeerId>,
}
impl DiscoveryConnectRequest {
	/// Test if any did discovery listening to this topic.
	///
	/// TODO: (perf) index?
	pub fn is_did_discovery_topic(&self, topic: &gossipsub::TopicHash) -> bool {
		for discovery in &self.discovery {
			match discovery {
				Discovery::DidDiscovery(item) =>
					if &did_discovery_topic(&item.network).hash() == topic {
						return true;
					},
				_ => {},
			}
		}
		return false;
	}
}

/// Active subscription listening for DID Discovery requests.
struct DidDiscoverySubscription {
	network: NetworkDidDiscovery,
	identity: PrivateIdentityBox,
}

#[derive(Debug, Clone)]
pub enum DiscoveryEvent {
	// Resolved { id: u64, request: Discovery, peers: BTreeMap<PeerId, Vec<Multiaddr>> },
	/// A peer to be discovered has connected.
	Connected { id: u64, peer: PeerId },

	/// A peer to be discovered has disconnected.
	Disconnected { id: u64, peer: PeerId },

	/// A discovery connect has timedout.
	Timeout { id: u64 },

	/// A resolve request to us via did discovery gossip.
	/// With an pre-computed DIDComm response.
	Resolve { from: Did, from_peer: PeerId, response: String },
}

pub struct DiscoveryState {
	/// Next discovery request id.
	next_id: u64,

	/// Active discovery requests.
	requests: BTreeMap<u64, DiscoveryConnectRequest>,

	/// Active subscription listening for DID Discovery requests.
	did_subscriptions: BTreeMap<TopicHash, Vec<DidDiscoverySubscription>>,

	/// Pending events.
	events: VecDeque<DiscoveryEvent>,

	/// Default discovery timeout.
	timeout: Duration,

	/// Default discovery max peers.
	max_peers: Option<u16>,
}
impl DiscoveryState {
	pub fn new(timeout: Duration, max_peers: Option<u16>) -> Self {
		Self {
			next_id: 1,
			requests: Default::default(),
			timeout,
			max_peers,
			events: Default::default(),
			did_subscriptions: Default::default(),
		}
	}

	/// Subscribe identity for DID Discovery.
	pub fn did_discovery_subscribe<B, P>(
		&mut self,
		swarm: &mut Swarm<B>,
		network: NetworkDidDiscovery,
		identity: P,
	) -> Result<(), anyhow::Error>
	where
		B: DiscoveryBehaviour,
		P: PrivateIdentity + Send + Sync + 'static,
	{
		let topic = did_discovery_topic(&network);

		// add
		self.did_subscriptions
			.entry(topic.hash())
			.or_insert(Default::default())
			.push(DidDiscoverySubscription { identity: Box::new(identity), network: network.clone() });

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
	pub fn connect<B: DiscoveryBehaviour>(
		&mut self,
		swarm: &mut Swarm<B>,
		discovery: impl IntoIterator<Item = Discovery>,
	) -> Result<(), anyhow::Error>
	where
		B:,
	{
		// id
		let id = self.next_id;
		self.next_id += 1;

		// request
		let request = DiscoveryConnectRequest {
			id,
			discovery: discovery.into_iter().collect(),
			start: Instant::now(),
			max_peers: self.max_peers,
			timeout: self.timeout,
			connected_peers: Default::default(),
		};
		self.requests.insert(id, request);
		let request = self.requests.get_mut(&id).unwrap();

		// connect
		for item in request.discovery.clone().into_iter() {
			match item {
				Discovery::DidDiscovery(item) => {
					did_discovery(swarm, &item)?;
				},
				Discovery::Rendezvous(_item) => {
					// TODO: implement
				},
				Discovery::Peer(item) => {
					peer(swarm, request, &mut self.events, &item)?;
				},
			}
		}

		Ok(())
	}

	/// Returns currently connected peers for an request id. If request can not be found return an emty set.
	pub fn peers(&self, id: u64) -> BTreeSet<PeerId> {
		self.requests.get(&id).map(|r| &r.connected_peers).cloned().unwrap_or_default()
	}

	/// Release (may disconnect) discovered peers.
	pub fn release(&mut self, id: u64) {
		self.requests.remove(&id);
	}

	/// Poll on events.
	pub fn poll(&mut self, _cx: &mut Context<'_>) -> Poll<DiscoveryEvent> {
		// TODO: timeouts

		// events
		if let Some(event) = self.events.pop_front() {
			return Poll::Ready(event);
		}

		// pending
		Poll::Pending
	}

	/// Handle swarm events.
	///
	/// Specifically:
	/// - Emit events for direct peer connections.
	/// - Forward behaviour events to handlers.
	pub fn on_swarm_event<B: DiscoveryBehaviour>(&mut self, event: &SwarmEvent<<B as NetworkBehaviour>::ToSwarm>) {
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
						.peer_requests()
						.filter(|(_, request_peer, _)| peer_id == request_peer)
						.map(|(request, _, _)| request)
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
						.peer_requests()
						.filter(|(_, request_peer, _)| peer_id == request_peer)
						.map(|(request, _, _)| request)
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
				if let Some(rendezvous_client_event) = B::rendezvous_client_event(behaviour_event) {
					self.on_rendezvous_client_event(rendezvous_client_event);
				}
			},
			_ => {},
		}
	}

	/// Iterate over all direct peer connection networks.
	fn peer_requests(&self) -> impl Iterator<Item = (u64, PeerId, &NetworkPeer)> {
		self.requests.iter().flat_map(|(request, discovery_request)| {
			discovery_request.discovery.iter().filter_map(|discovery| match discovery {
				Discovery::Peer(network) => Some((
					*request,
					PeerId::from_bytes(&network.peer).expect("Discovery::requests to only contain only valid peer ids"),
					network,
				)),
				_ => None,
			})
		})
	}

	/// Handle GossipSub events.
	///
	/// Specifically:
	/// - Receiving DID Discovery messages addressed to us.
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
							Err(err) => {
								#[cfg(debug_assertions)]
								tracing::debug!(?err, "recevice-invalid-message");
								return;
							},
						};

						// try to recevice
						let result = didcomm_recevice(data, subscriptions.iter().map(|s| &s.identity));
						if let Some((request_header, _, identity)) = result {
							if let Some(didcomm_private) = identity.didcomm_private() {
								match did_discovery_resolve(
									&mut self.events,
									&didcomm_private,
									request_from_peer,
									request_header,
								) {
									Ok(()) => {},
									Err(err) => {
										tracing::warn!(?err, "did-discovery-resolve-failed");
									},
								}
							}
						}
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
	fn on_didcomm_event(&mut self, event: &didcomm::Event) {
		match event {
			didcomm::Event::Received { peer_id, message } =>
				if let Some(message) = message.json() {
					let identities = self
						.did_subscriptions
						.iter()
						.flat_map(|(_, subscriptions)| subscriptions.iter().map(|s| &s.identity));
					if let Some((header, body, identity)) = didcomm_recevice(message, identities) {
						if header.message_type == "diddiscovery" {
							// TODO
						}
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
	fn on_mdns_event(&mut self, event: &mdns::Event) {}

	fn on_rendezvous_client_event(&mut self, event: &rendezvous::client::Event) {
		// TODO
	}
}
impl Stream for DiscoveryState {
	type Item = DiscoveryEvent;

	/// Note: This stream is infinite.
	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		self.as_mut().poll(cx).map(Some)
	}
}
/// As we produce the events in an infinite manner the stream will never be terminated.
impl FusedStream for DiscoveryState {
	fn is_terminated(&self) -> bool {
		false
	}
}

pub trait DiscoveryBehaviour: NetworkBehaviour {
	fn gossipsub_mut(&mut self) -> &mut gossipsub::Behaviour;
	fn didcomm_mut(&mut self) -> &mut didcomm::Behaviour;
	fn rendezvous_client_mut(&mut self) -> Option<&mut rendezvous::client::Behaviour>;
	// fn mdns_mut(&mut self) -> Option<&mut mdns::tokio::Behaviour>;
	// fn kad_mut(&mut self) -> Option<&mut kad::Behaviour<kad::store::MemoryStore>>;
	fn gossipsub_event(event: &<Self as NetworkBehaviour>::ToSwarm) -> Option<&gossipsub::Event>;
	fn didcomm_event(event: &<Self as NetworkBehaviour>::ToSwarm) -> Option<&didcomm::Event>;
	fn rendezvous_client_event(event: &<Self as NetworkBehaviour>::ToSwarm) -> Option<&rendezvous::client::Event>;
}

/// Respond to an did discovery resolve request.
fn did_discovery_resolve(
	events: &mut VecDeque<DiscoveryEvent>,
	identity: &DidCommPrivateContext,
	request_from_peer: PeerId,
	request: DidCommHeader,
) -> Result<(), anyhow::Error> {
	let request_from = request.from.ok_or(anyhow!("Missing from header field"))?;

	// response
	let mut response = DidCommHeader::new();
	response.message_type = "diddiscovery".to_owned();
	response.thid = Some(request.id);
	response.from = Some(identity.did().to_owned());
	response.to.insert(request_from.clone());

	// message
	let message = identity.jws(response, "null")?;

	// send
	events.push_back(DiscoveryEvent::Resolve { from: request_from, from_peer: request_from_peer, response: message });

	// result
	Ok(())
}

/// Try to recevice a message (data) with one of the supplied identities.
fn didcomm_recevice<'a>(
	data: &str,
	identities: impl Iterator<Item = &'a PrivateIdentityBox>,
) -> Option<(DidCommHeader, String, &'a PrivateIdentityBox)> {
	for identity in identities {
		if let Some(didcomm_private) = identity.didcomm_private() {
			match didcomm_private.jwe_receive(data) {
				Ok((header, body)) => return Some((header, body, identity)),
				Err(err) => {
					// note: this will happen on purpose because we check the message against all identities and only
					// one will match.
					#[cfg(debug_assertions)]
					tracing::debug!(?err, "jwe-receive-failed");
				},
			}
		}
	}
	None
}

/// Get did discovery gossipsub topic.
fn did_discovery_topic(network: &NetworkDidDiscovery) -> gossipsub::IdentTopic {
	gossipsub::IdentTopic::new(did_discovery_topic_str(network))
}

/// Get did discovery gossipsub topic as string.
fn did_discovery_topic_str(did_discovery: &NetworkDidDiscovery) -> &str {
	did_discovery.topic.as_deref().unwrap_or("co-contact")
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
	item: &NetworkPeer,
) -> Result<(), anyhow::Error> {
	let peer: PeerId = PeerId::from_bytes(&item.peer)?;
	let addresses = item
		.addresses
		.iter()
		.map(|address| Multiaddr::from_str(&address))
		.collect::<Result<BTreeSet<_>, _>>()?;

	// already connected?
	if swarm.is_connected(&peer) {
		peer_connected(request, events, peer);
		return Ok(());
	}

	// dail
	let opts = DialOpts::peer_id(peer)
		.addresses(addresses.clone().into_iter().collect())
		.build();
	swarm.dial(opts)?;

	// done
	Ok(())
}

fn peer_connected(request: &mut DiscoveryConnectRequest, events: &mut VecDeque<DiscoveryEvent>, peer: PeerId) {
	request.connected_peers.insert(peer);
	events.push_back(DiscoveryEvent::Connected { id: request.id, peer });
}

fn peer_disconnected(request: &mut DiscoveryConnectRequest, events: &mut VecDeque<DiscoveryEvent>, peer: PeerId) {
	request.connected_peers.remove(&peer);
	events.push_back(DiscoveryEvent::Disconnected { id: request.id, peer });
}

#[cfg(test)]
mod tests {
	use super::{Discovery, DiscoveryBehaviour};
	use crate::{didcomm, discovery::did_discovery::DidDiscovery, heads, DiscoveryEvent, DiscoveryState};
	use co_identity::DidKeyIdentity;
	use co_primitives::{BlockSerializer, CoId, NetworkDidDiscovery};
	use futures::{join, select, FutureExt, StreamExt};
	use libipld::Cid;
	use libp2p::{
		gossipsub::{self, IdentTopic},
		identity::Keypair,
		noise, rendezvous,
		swarm::{dial_opts::DialOpts, NetworkBehaviour, SwarmEvent},
		tcp, yamux, Multiaddr, PeerId, Swarm, SwarmBuilder,
	};
	use std::{time::Duration, vec};

	#[derive(NetworkBehaviour)]
	struct TestBehaviour {
		didcomm: didcomm::Behaviour,
		gossipsub: gossipsub::Behaviour,
	}
	impl TestBehaviour {
		pub fn new(keypair: Keypair) -> Self {
			let gossipsub_config = gossipsub::ConfigBuilder::default()
				.max_transmit_size(256 * 1024)
				.build()
				.expect("valid config");
			let gossipsub_behaviour =
				gossipsub::Behaviour::new(gossipsub::MessageAuthenticity::Signed(keypair), gossipsub_config)
					.expect("gossipsub");
			let didcomm_behaviour = didcomm::Behaviour::new(didcomm::Config { auto_dail: false });
			TestBehaviour { didcomm: didcomm_behaviour, gossipsub: gossipsub_behaviour }
		}
	}
	impl DiscoveryBehaviour for TestBehaviour {
		fn gossipsub_mut(&mut self) -> &mut gossipsub::Behaviour {
			&mut self.gossipsub
		}

		fn didcomm_mut(&mut self) -> &mut didcomm::Behaviour {
			&mut self.didcomm
		}

		fn rendezvous_client_mut(&mut self) -> Option<&mut rendezvous::client::Behaviour> {
			None
		}

		fn gossipsub_event(event: &<Self as NetworkBehaviour>::ToSwarm) -> Option<&gossipsub::Event> {
			match event {
				TestBehaviourEvent::Gossipsub(e) => Some(e),
				_ => None,
			}
		}

		fn didcomm_event(event: &<Self as NetworkBehaviour>::ToSwarm) -> Option<&didcomm::Event> {
			match event {
				TestBehaviourEvent::Didcomm(e) => Some(e),
				_ => None,
			}
		}

		fn rendezvous_client_event(event: &<Self as NetworkBehaviour>::ToSwarm) -> Option<&rendezvous::client::Event> {
			None
		}
	}

	struct Peer {
		peer_id: PeerId,
		addr: Multiaddr,
		swarm: Swarm<TestBehaviour>,
	}
	impl Peer {
		fn new() -> Self {
			let mut swarm = SwarmBuilder::with_new_identity()
				.with_tokio()
				.with_tcp(tcp::Config::default(), noise::Config::new, yamux::Config::default)
				.unwrap()
				.with_behaviour(|k| Ok(TestBehaviour::new(k.clone())))
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

		async fn next(&mut self) -> Option<TestBehaviourEvent> {
			loop {
				let ev = self.swarm.next().await?;
				if let SwarmEvent::Behaviour(event) = ev {
					return Some(event);
				}
			}
		}

		fn on_swarm_event(&self, discovery: &mut DiscoveryState, event: &SwarmEvent<TestBehaviourEvent>) {
			// foward gossip
			match event {
				SwarmEvent::Behaviour(TestBehaviourEvent::Gossipsub(event)) => {
					discovery.on_gossip_event(&event);
				},
				SwarmEvent::Behaviour(TestBehaviourEvent::Didcomm(event)) => {
					discovery.on_didcomm_event(&event);
				},
				// SwarmEvent::Behaviour(TestBehaviourEvent::Mdns(event)) => {
				// 	discovery1.on_mdns_event(&event);
				// },
				_ => {},
			}
		}

		fn on_discovery_event(
			&mut self,
			discovery: &mut DiscoveryState,
			event: &DiscoveryEvent,
		) -> Option<(u64, PeerId)> {
			match event {
				DiscoveryEvent::Resolve { from: _, from_peer, response } => {
					// respond
					self.swarm()
						.behaviour_mut()
						.didcomm_mut()
						.send(from_peer, response.as_str().into());
					None
				},
				DiscoveryEvent::Connected { id, peer } => Some((*id, *peer)),
				_ => None,
			}
		}
	}

	//#[tracing_test::traced_test]
	//#[test_log::test(tokio::test)]
	#[tokio::test]
	async fn test_subscribe() {
		tracing_subscriber::fmt()
			.with_env_filter(tracing_subscriber::EnvFilter::new(format!(
				"{}=trace",
				module_path!().split(":").next().expect("module path")
			)))
			.try_init()
			.ok();

		// peers
		let mut peer1 = Peer::new();
		let mut peer2 = Peer::new();
		peer2.add_address(&peer1);

		// identities
		let did1 = DidKeyIdentity::generate_x25519(Some(&vec![1; 32]));
		let did2 = DidKeyIdentity::generate_x25519(Some(&vec![2; 32]));

		// states
		let mut discovery1 = DiscoveryState::new(Duration::from_secs(10), None);
		let mut discovery2 = DiscoveryState::new(Duration::from_secs(10), None);

		// peer1: subscribe
		discovery1
			.did_discovery_subscribe(peer1.swarm(), NetworkDidDiscovery::default(), did1.clone())
			.unwrap();

		// peer2: subscribe
		discovery2
			.did_discovery_subscribe(peer2.swarm(), NetworkDidDiscovery::default(), did2.clone())
			.unwrap();

		// wait subscribed
		let (subscribe1, subscribe2) = join!(peer1.next(), peer2.next());
		match subscribe1 {
			Some(TestBehaviourEvent::Gossipsub(gossipsub::Event::Subscribed { peer_id, topic }))
				if topic == IdentTopic::new("co-contact").hash() && peer_id == peer2.peer_id => {},
			event => panic!("unexpected event: {:?}", event),
		}
		match subscribe2 {
			Some(TestBehaviourEvent::Gossipsub(gossipsub::Event::Subscribed { peer_id, topic }))
				if topic == IdentTopic::new("co-contact").hash() && peer_id == peer1.peer_id => {},
			event => panic!("unexpected event: {:?}", event),
		}

		// peer2: connect
		discovery2
			.connect(
				peer2.swarm(),
				vec![DidDiscovery::create(
					&did2,
					&did1,
					NetworkDidDiscovery::default(),
					"diddiscovery-resolve".to_owned(),
				)
				.unwrap()
				.into()],
			)
			.unwrap();

		// run
		let mut max_events = 20;
		loop {
			max_events -= 1;
			if max_events == 0 {
				break;
			}

			// let mut discoverd = None;
			select! {
				event = peer1.swarm().next() => {
					tracing::info!(?event, "peer1");
					discovery1.on_swarm_event::<TestBehaviour>(&event.unwrap());
				},
				event = peer2.swarm().next() => {
					tracing::info!(?event, "peer2");
					discovery2.on_swarm_event::<TestBehaviour>(&event.unwrap());
				},
				event = discovery1.next() => {
					tracing::info!(?event, "discovery1");
					peer1.on_discovery_event(&mut discovery1, &event.unwrap());
				},
				event = discovery2.next() => {
					tracing::info!(?event, "discovery2");
					if let Some((id, peer)) = peer2.on_discovery_event(&mut discovery2, &event.unwrap()) {
						assert_eq!(peer, peer2.peer_id());
						break;
					}
				},
			};
		}
	}
}
