use crate::{
	didcomm, heads::heads_message::HeadsMessage, types::layer_behaviour::LayerBehaviour, DidcommBehaviourProvider,
	GossipsubBehaviourProvider,
};
use co_identity::{DidCommHeader, Message};
use co_primitives::{CoId, NetworkCoHeads};
use libipld::Cid;
use libp2p::{
	gossipsub::{self, IdentTopic, PublishError, TopicHash},
	swarm::{NetworkBehaviour, SwarmEvent},
	PeerId, Swarm,
};
use std::{
	collections::{BTreeMap, BTreeSet, VecDeque},
	task::{Context, Poll},
	time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum Event {
	/// Receviced new heads from some peer.
	ReceivedHeads {
		/// The CO ID.
		co: CoId,

		/// Received Heads.
		heads: BTreeSet<Cid>,

		/// Received from peer.
		peer_id: Option<PeerId>,

		/// Protocol requested a response.
		/// Typically,
		/// - false when received via co-heads (gossipsub).
		/// - true when received via direct didcomm.
		response: bool,
	},
	//
	// /// Sent heads to an peer.
	// SentHeads { co: CoId, heads: BTreeSet<Cid>, peer_id: PeerId },

	// /// Subscribed for heads.
	// Subscribed { co: CoId },

	// /// Unsubscribed for heads.
	// Unsubscribed { co: CoId },
}

#[derive(Debug, Clone)]
pub enum HeadsEvent {
	GenerateEvent(Event),
	Publish(PublishHeads),
}

#[derive(Debug, Clone)]
pub struct PublishHeads {
	network: NetworkCoHeads,
	co: CoId,
	heads: BTreeSet<Cid>,
}

pub struct HeadsState {
	heads: BTreeSet<TopicHash>,
	events: VecDeque<HeadsEvent>,
	pending_heads: BTreeMap<TopicHash, Vec<PublishHeads>>,
}
impl HeadsState {
	pub fn new() -> Self {
		Self { heads: Default::default(), events: Default::default(), pending_heads: Default::default() }
	}

	/// Subscribe to CO gossip.
	/// Returns `true` if a new subscription has been made, `false` is was already subscribed.
	pub fn subscribe<B: NetworkBehaviour + GossipsubBehaviourProvider>(
		&mut self,
		swarm: &mut Swarm<B>,
		network: &NetworkCoHeads,
		co: &CoId,
	) -> Result<bool, anyhow::Error> {
		let topic = self.to_topic(network, co);
		self.heads.insert(topic.hash());
		Ok(swarm.behaviour_mut().gossipsub_mut().subscribe(&topic)?)
	}

	pub fn unsubscribe<B: NetworkBehaviour + GossipsubBehaviourProvider>(
		&mut self,
		swarm: &mut Swarm<B>,
		network: &NetworkCoHeads,
		co: &CoId,
	) -> Result<bool, anyhow::Error> {
		let topic = self.to_topic(network, co);
		self.heads.remove(&topic.hash());
		Ok(swarm.behaviour_mut().gossipsub_mut().unsubscribe(&topic)?)
	}

	pub fn publish<B: NetworkBehaviour + GossipsubBehaviourProvider>(
		&mut self,
		swarm: &mut Swarm<B>,
		network: &NetworkCoHeads,
		co: &CoId,
		heads: &BTreeSet<Cid>,
	) -> Result<(), anyhow::Error> {
		let topic = self.to_topic(network, co);
		let message = HeadsMessage::Heads(co.clone(), heads.clone());
		let data = serde_ipld_dagcbor::to_vec(&message)?;
		match swarm.behaviour_mut().gossipsub_mut().publish(topic, data) {
			Ok(_) => Ok(()),
			Err(PublishError::InsufficientPeers) => {
				// insert as pending by only keeping latest publish by every co
				let pending = self
					.pending_heads
					.entry(self.to_topic(network, co).hash())
					.or_insert(Default::default());
				pending.retain(|item| &item.co != co);
				pending.push(PublishHeads { network: network.clone(), co: co.clone(), heads: heads.clone() });
				Ok(())
			},
			Err(e) => Err(e.into()),
		}
	}

	/// Send heads to peers.
	/// Peers will answer with own heads if they are different.
	/// However the response is not implemented by this protocol but by the caller.
	/// TODO: identity: need to sign?
	pub fn heads<B: NetworkBehaviour + DidcommBehaviourProvider>(
		&mut self,
		swarm: &mut Swarm<B>,
		co: &CoId,
		heads: BTreeSet<Cid>,
		peers: impl IntoIterator<Item = PeerId>,
	) -> Result<(), anyhow::Error> {
		let time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Valid time").as_secs();
		let header = DidCommHeader {
			created_time: Some(time),
			expires_time: Some(time + 120),
			from: None,
			id: Uuid::new_v4().into(),
			message_type: format!("co-heads/1.0.0"),
			pthid: None,
			thid: None,
			to: Default::default(),
		};
		let message = Message { header, body: HeadsMessage::Heads(co.clone(), heads) };
		let data = message.cbor()?;
		for peer in peers {
			swarm
				.behaviour_mut()
				.didcomm_mut()
				.send(&peer, didcomm::Message::Message(data.clone()));
		}
		Ok(())
	}

	fn to_topic(&self, network: &NetworkCoHeads, id: &CoId) -> gossipsub::IdentTopic {
		IdentTopic::new(network.topic.clone().unwrap_or_else(|| format!("co-{}", id)))
	}

	fn on_gossip_event(&mut self, event: &gossipsub::Event) {
		match event {
			gossipsub::Event::Message { propagation_source: _, message_id: _, message } => {
				if self.heads.contains(&message.topic) {
					// TODO(metric): add metrics when receive invalid message?
					let heads_message: Option<HeadsMessage> = serde_ipld_dagcbor::from_slice(&message.data).ok();
					match heads_message {
						Some(HeadsMessage::Heads(co, heads)) =>
							self.events.push_back(HeadsEvent::GenerateEvent(Event::ReceivedHeads {
								co,
								heads,
								peer_id: message.source,
								response: false,
							})),
						None => {},
					}
				}
			},
			gossipsub::Event::Subscribed { peer_id: _, topic } => {
				// when we have at least on subscriber trigger pending publish events
				if let Some(pending) = self.pending_heads.remove(topic) {
					self.events.extend(pending.into_iter().map(|item| HeadsEvent::Publish(item)));
				}
			},
			_ => {},
		}
	}

	fn on_didcomm_event(&mut self, event: &didcomm::Event) {
		match event {
			didcomm::Event::Received { peer_id, message } =>
				if let Some(cbor) = message.cbor() {
					// TODO(metric): add metrics when receive invalid message?
					let message = Message::<HeadsMessage>::from_cbor(cbor).ok();
					match message.map(|message| message.body) {
						Some(HeadsMessage::Heads(co, heads)) =>
							self.events.push_back(HeadsEvent::GenerateEvent(Event::ReceivedHeads {
								co,
								heads,
								peer_id: Some(*peer_id),
								response: true,
							})),
						_ => {},
					}
				},
			_ => {},
			// didcomm::Event::Sent { peer_id, message } => match message {
			// didcomm::Message::Message(data) => {
			// 	let heads_message: HeadsMessage = match Message::<HeadsMessage>::from_cbor(&data) {
			// 		Ok(m) => m.body,
			// 		Err(err) => {
			// 			panic!("BUG: can not deserialize just serialized message: {:?}", err);
			// 		},
			// 	};
			// 	match heads_message {
			// 		HeadsMessage::Heads(co, heads) =>
			// 			Poll::Ready(ToSwarm::GenerateEvent(Event::SentHeads { co, heads, peer_id })),
			// 	}
			// },
			// },
		}
	}
}
impl<B> LayerBehaviour<B> for HeadsState
where
	B: NetworkBehaviour + DidcommBehaviourProvider + GossipsubBehaviourProvider,
{
	type ToSwarm = Event;
	type ToLayer = HeadsEvent;

	fn on_swarm_event(&mut self, event: &SwarmEvent<<B as NetworkBehaviour>::ToSwarm>) {
		match event {
			SwarmEvent::Behaviour(behaviour_event) => {
				if let Some(gossip_event) = B::gossipsub_event(behaviour_event) {
					self.on_gossip_event(gossip_event);
				}
				if let Some(didcomm_event) = B::didcomm_event(behaviour_event) {
					self.on_didcomm_event(didcomm_event);
				}
			},
			_ => {},
		}
	}

	fn on_layer_event(&mut self, swarm: &mut Swarm<B>, event: Self::ToLayer) -> Option<Self::ToSwarm> {
		match event {
			HeadsEvent::GenerateEvent(event) => Some(event),
			HeadsEvent::Publish(publish) => {
				match self.publish(swarm, &publish.network, &publish.co, &publish.heads) {
					Ok(()) => {},
					Err(err) => {
						// todo: generate some error event?
						tracing::warn!(?err, ?publish, "heads-publish-failed");
					},
				}
				None
			},
		}
	}

	fn poll(&mut self, _cx: &mut Context<'_>) -> Poll<Self::ToLayer> {
		// events
		if let Some(event) = self.events.pop_front() {
			return Poll::Ready(event);
		}

		// pending
		Poll::Pending
	}
}

#[cfg(test)]
mod tests {
	use crate::{
		didcomm,
		heads::{self, HeadsState},
		DidcommBehaviourProvider, GossipsubBehaviourProvider, Layer, LayerBehaviour,
	};
	use co_primitives::{BlockSerializer, CoId, NetworkCoHeads};
	use futures::{FutureExt, StreamExt};
	use libipld::Cid;
	use libp2p::{
		gossipsub,
		identity::Keypair,
		noise,
		swarm::{dial_opts::DialOpts, NetworkBehaviour},
		tcp, yamux, Multiaddr, PeerId, Swarm, SwarmBuilder,
	};
	use std::{collections::BTreeSet, time::Duration, vec};
	use tokio::select;

	#[derive(NetworkBehaviour)]
	struct TestBehaviour {
		gossipsub: gossipsub::Behaviour,
		didcomm: didcomm::Behaviour,
	}
	impl TestBehaviour {
		fn new(keypair: Keypair) -> Self {
			let gossipsub_config = gossipsub::ConfigBuilder::default()
				.max_transmit_size(256 * 1024)
				.build()
				.expect("valid config");
			let gossipsub_behaviour =
				gossipsub::Behaviour::new(gossipsub::MessageAuthenticity::Signed(keypair), gossipsub_config)
					.expect("gossipsub");
			let didcomm_behaviour = didcomm::Behaviour::new(didcomm::Config { auto_dail: false });
			Self { didcomm: didcomm_behaviour, gossipsub: gossipsub_behaviour }
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

		fn add_address(&mut self, _co: CoId, peer: &Peer) {
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
	async fn test_subscribe() {
		let co = CoId::new("test");

		// test data
		let test = BlockSerializer::default().serialize(&"test").unwrap();
		let hello = BlockSerializer::default().serialize(&"hello").unwrap();

		// heads
		let mut h1: BTreeSet<Cid> = BTreeSet::new();
		h1.insert(test.cid().clone());
		let mut h2: BTreeSet<Cid> = BTreeSet::new();
		h2.insert(hello.cid().clone());

		// peers
		let mut peer1 = Peer::new();
		let mut peer2 = Peer::new();
		peer2.add_address(co.clone(), &peer1);

		// layer
		let mut heads1 = Layer::new(peer1.swarm().behaviour(), HeadsState::new());
		let mut heads2 = Layer::new(peer2.swarm().behaviour(), HeadsState::new());

		// peer1: subscribe
		heads1
			.layer_mut()
			.subscribe(peer1.swarm(), &NetworkCoHeads::default(), &co)
			.unwrap();

		// peer2: subscribe
		heads2
			.layer_mut()
			.subscribe(peer1.swarm(), &NetworkCoHeads::default(), &co)
			.unwrap();

		// // wait until both are subscribed
		// let (subscribe1, subscribe2) = join!(peer1.next(), peer2.next());
		// match subscribe1 {
		// 	Some(gossipsub::Event::Subscribed { co: event_co, peer_id })
		// 		if co == event_co && peer_id == peer2.peer_id => {},
		// 	event => panic!("unexpected event: {:?}", event),
		// }
		// match subscribe2 {
		// 	Some(heads::Event::Subscribed { co: event_co, peer_id }) if co == event_co && peer_id == peer1.peer_id => {
		// 	},
		// 	event => panic!("unexpected event: {:?}", event),
		// }

		// peer2: update heads
		heads2
			.layer_mut()
			.publish(peer2.swarm(), &NetworkCoHeads::default(), &co, &h2)
			.unwrap();

		// run
		// note: we also neeed to run peer1 to advance its state
		select! {
			event = peer1.swarm().next() => {
				heads1.on_swarm_event(&event.unwrap());
			},
			event = peer2.swarm().next() => {
				heads2.on_swarm_event(&event.unwrap());
			},
			event = heads1.next() => {
				tracing::info!(?event, "heads1");
				match heads1.on_layer_event(peer2.swarm(), event.unwrap()) {
					Some(heads::Event::ReceivedHeads { co: event_co, heads: event_heads, peer_id: _, response: _ }) if co == event_co && h2 == event_heads => {},
					event => panic!("unexpected event: {:?}", event),
				}
			},
			event = heads2.next() => {
				tracing::info!(?event, "heads2");
				heads2.on_layer_event(peer1.swarm(), event.unwrap());
			},
		};
	}

	#[tokio::test]
	async fn test_heads() {
		let co = CoId::new("test");

		// test data
		let test = BlockSerializer::default().serialize(&"test").unwrap();
		let hello = BlockSerializer::default().serialize(&"hello").unwrap();

		// heads
		let mut h1: BTreeSet<Cid> = BTreeSet::new();
		h1.insert(test.cid().clone());
		let mut h2: BTreeSet<Cid> = BTreeSet::new();
		h2.insert(hello.cid().clone());

		// peers
		let mut peer1 = Peer::new();
		let mut peer2 = Peer::new();
		tracing::info!(peer_id = ?peer1.swarm.local_peer_id(), "peer1");
		tracing::info!(peer_id = ?peer2.swarm.local_peer_id(), "peer2");
		peer2.add_address(co.clone(), &peer1);

		// layer
		let mut heads1 = Layer::new(peer1.swarm().behaviour(), HeadsState::new());
		let mut heads2 = Layer::new(peer2.swarm().behaviour(), HeadsState::new());

		// peer2: heads
		heads2
			.layer_mut()
			.heads(peer2.swarm(), &co, h2.clone(), vec![peer1.peer_id.clone()])
			.unwrap();

		// wait for sent and received event
		loop {
			select! {
				event = peer1.swarm().next() => {
					heads1.on_swarm_event(&event.unwrap());
				},
				event = peer2.swarm().next() => {
					heads2.on_swarm_event(&event.unwrap());
				},
				event = heads1.next() => {
					tracing::info!(?event, "heads1");
					match heads1.on_layer_event(peer2.swarm(), event.unwrap()) {
						Some(heads::Event::ReceivedHeads { co: event_co, heads, peer_id, response }) => {
							assert_eq!(co, event_co);
							assert_eq!(Some(peer2.peer_id), peer_id);
							assert_eq!(h2, heads);
							assert_eq!(true, response);
							break;
						},
						event => panic!("unexpected event: {:?}", event),
					}
				},
				event = heads2.next() => {
					tracing::info!(?event, "heads2");
					heads2.on_layer_event(peer1.swarm(), event.unwrap());
				},
			}
		}
	}
}
