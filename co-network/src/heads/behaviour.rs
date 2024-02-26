use crate::{didcomm, heads::message::HeadsMessage, Message};
use co_primitives::CoId;
use libipld::Cid;
use libp2p::{
	core::Endpoint,
	gossipsub::{self, IdentTopic, PublishError},
	identity::Keypair,
	swarm::{ConnectionDenied, ConnectionId, FromSwarm, NetworkBehaviour, THandlerInEvent, THandlerOutEvent, ToSwarm},
	Multiaddr, PeerId,
};
use std::{
	collections::{BTreeMap, BTreeSet},
	task::{Context, Poll},
	time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

#[derive(Debug)]
pub enum Event {
	/// Receviced new heads from some peer.
	ReceivedHeads { co: CoId, heads: BTreeSet<Cid>, peer_id: Option<PeerId>, response: bool },

	/// Subscribed for heads.
	Subscribed { co: CoId, peer_id: PeerId },

	/// Unsubscribed for heads.
	Unsubscribed { co: CoId, peer_id: PeerId },

	/// Forwarded gossipsub
	/// Todo: remove?
	Gossipsub(gossipsub::Event),

	/// Forwarded didcomm
	/// Todo: remove?
	Didcomm(didcomm::Event),
}
impl From<InnerBehaviourEvent> for Event {
	fn from(value: InnerBehaviourEvent) -> Self {
		match value {
			InnerBehaviourEvent::Gossipsub(e) => Event::Gossipsub(e),
			InnerBehaviourEvent::Didcomm(e) => Event::Didcomm(e),
		}
	}
}

pub struct Behaviour {
	inner: InnerBehaviour,
	explicit_peers: BTreeMap<CoId, BTreeSet<PeerId>>,
}
impl Behaviour {
	pub fn new(keypair: Keypair) -> Self {
		let gossipsub_config = gossipsub::ConfigBuilder::default()
			.max_transmit_size(256 * 1024)
			.build()
			.expect("valid config");
		let gossipsub_behaviour =
			gossipsub::Behaviour::new(gossipsub::MessageAuthenticity::Signed(keypair), gossipsub_config)
				.expect("gossipsub");
		let didcomm_behaviour = didcomm::Behaviour::new(didcomm::Config { auto_dail: false });
		Self {
			inner: InnerBehaviour { didcomm: didcomm_behaviour, gossipsub: gossipsub_behaviour },
			explicit_peers: Default::default(),
		}
	}

	pub fn add_explicit_peer(&mut self, co: CoId, peer_id: PeerId) {
		self.inner.gossipsub.add_explicit_peer(&peer_id);
		self.explicit_peers.entry(co).or_default().insert(peer_id);
	}

	/// Subscribe to CO gossip.
	/// Returns `true` if a new subscription has been made, `false` is was already subscribed.
	pub fn subscribe(&mut self, co: &CoId) -> Result<bool, anyhow::Error> {
		Ok(self.inner.gossipsub.subscribe(&self.to_topic(co))?)
	}

	pub fn unsubscribe(&mut self, co: &CoId) -> Result<bool, anyhow::Error> {
		Ok(self.inner.gossipsub.unsubscribe(&self.to_topic(co))?)
	}

	pub fn publish_heads(&mut self, co: &CoId, heads: BTreeSet<Cid>) -> Result<bool, anyhow::Error> {
		let message = HeadsMessage::Heads(co.clone(), heads);
		let data = serde_ipld_dagcbor::to_vec(&message)?;
		match self.inner.gossipsub.publish(self.to_topic(co), data) {
			Ok(_) => Ok(true),
			Err(PublishError::InsufficientPeers) => Ok(false),
			Err(e) => Err(e.into()),
		}
	}

	/// Send heads to peers.
	/// Peers will answer with own heads if they are different.
	/// However the response is not implemented by this protocol but by the caller.
	/// TODO: identity: need to sign?
	pub fn heads(
		&mut self,
		co: &CoId,
		heads: BTreeSet<Cid>,
		peers: impl Iterator<Item = PeerId>,
	) -> Result<(), anyhow::Error> {
		let time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Valid time").as_secs();
		let message = Message {
			body: HeadsMessage::Heads(co.clone(), heads),
			created_time: Some(time),
			expires_time: Some(time + 120),
			from: None,
			id: Uuid::new_v4().into(),
			message_type: "co/heads".to_owned(),
			pthid: None,
			thid: None,
			to: Default::default(),
		};
		let data = message.cbor()?;
		for peer in peers {
			self.inner.didcomm.send(&peer, didcomm::Message::Message(data.clone()));
		}
		Ok(())
	}

	fn to_topic(&self, id: &CoId) -> gossipsub::IdentTopic {
		IdentTopic::new(AsRef::<str>::as_ref(id))
	}

	fn to_co_id(&self, topic: &gossipsub::TopicHash) -> CoId {
		topic.as_str().into()
	}
}
impl NetworkBehaviour for Behaviour {
	type ConnectionHandler = <InnerBehaviour as NetworkBehaviour>::ConnectionHandler;
	type ToSwarm = Event;

	fn handle_established_inbound_connection(
		&mut self,
		connection_id: ConnectionId,
		peer: PeerId,
		local_addr: &Multiaddr,
		remote_addr: &Multiaddr,
	) -> Result<libp2p::swarm::THandler<Self>, ConnectionDenied> {
		self.inner
			.handle_established_inbound_connection(connection_id, peer, local_addr, remote_addr)
	}

	fn handle_established_outbound_connection(
		&mut self,
		connection_id: ConnectionId,
		peer: PeerId,
		addr: &Multiaddr,
		role_override: Endpoint,
	) -> Result<<Self as NetworkBehaviour>::ConnectionHandler, ConnectionDenied> {
		self.inner
			.handle_established_outbound_connection(connection_id, peer, addr, role_override)
	}

	fn on_swarm_event(&mut self, event: FromSwarm) {
		self.inner.on_swarm_event(event)
	}

	fn on_connection_handler_event(
		&mut self,
		peer_id: PeerId,
		connection_id: ConnectionId,
		event: THandlerOutEvent<Self>,
	) {
		self.inner.on_connection_handler_event(peer_id, connection_id, event)
	}

	fn poll(&mut self, cx: &mut Context<'_>) -> Poll<ToSwarm<Self::ToSwarm, THandlerInEvent<Self>>> {
		match self.inner.poll(cx) {
			// didcomm message
			Poll::Ready(ToSwarm::GenerateEvent(InnerBehaviourEvent::Didcomm(didcomm::Event::Received {
				peer_id,
				message,
			}))) => match message {
				didcomm::Message::Message(data) => {
					let heads_message: HeadsMessage = match serde_ipld_dagcbor::from_slice(&data) {
						Ok(m) => m,
						Err(err) => {
							tracing::warn!(?err, "received-invalid-message");
							return Poll::Pending;
						},
					};
					match heads_message {
						HeadsMessage::Heads(co, heads) => Poll::Ready(ToSwarm::GenerateEvent(Event::ReceivedHeads {
							co,
							heads,
							peer_id: Some(peer_id),
							response: true,
						})),
					}
				},
			},

			// gossip message
			Poll::Ready(ToSwarm::GenerateEvent(InnerBehaviourEvent::Gossipsub(gossipsub::Event::Message {
				propagation_source: _,
				message_id: _,
				message,
			}))) => {
				let heads_message: HeadsMessage = match serde_ipld_dagcbor::from_slice(&message.data) {
					Ok(m) => m,
					Err(err) => {
						tracing::warn!(?err, ?message.topic, "received-invalid-message");
						return Poll::Pending;
					},
				};
				match heads_message {
					HeadsMessage::Heads(co, heads) => Poll::Ready(ToSwarm::GenerateEvent(Event::ReceivedHeads {
						co,
						heads,
						peer_id: message.source,
						response: false,
					})),
				}
			},

			// subscribed
			Poll::Ready(ToSwarm::GenerateEvent(InnerBehaviourEvent::Gossipsub(gossipsub::Event::Subscribed {
				peer_id,
				topic,
			}))) => Poll::Ready(ToSwarm::GenerateEvent(Event::Subscribed { co: self.to_co_id(&topic), peer_id })),

			// unsubscribed
			Poll::Ready(ToSwarm::GenerateEvent(InnerBehaviourEvent::Gossipsub(gossipsub::Event::Unsubscribed {
				peer_id,
				topic,
			}))) => Poll::Ready(ToSwarm::GenerateEvent(Event::Unsubscribed { co: self.to_co_id(&topic), peer_id })),

			// forward
			Poll::Ready(event) => Poll::Ready(event.map_out(|event| event.into())),

			// pending
			Poll::Pending => Poll::Pending,
		}
	}

	fn handle_pending_inbound_connection(
		&mut self,
		connection_id: ConnectionId,
		local_addr: &Multiaddr,
		remote_addr: &Multiaddr,
	) -> Result<(), ConnectionDenied> {
		self.inner
			.handle_pending_inbound_connection(connection_id, local_addr, remote_addr)
	}

	fn handle_pending_outbound_connection(
		&mut self,
		connection_id: ConnectionId,
		maybe_peer: Option<PeerId>,
		addresses: &[Multiaddr],
		effective_role: Endpoint,
	) -> Result<Vec<Multiaddr>, ConnectionDenied> {
		self.inner
			.handle_pending_outbound_connection(connection_id, maybe_peer, addresses, effective_role)
	}
}

#[derive(NetworkBehaviour)]
pub struct InnerBehaviour {
	gossipsub: gossipsub::Behaviour,
	didcomm: didcomm::Behaviour,
}

#[cfg(test)]
mod tests {
	use crate::heads;
	use co_primitives::{BlockSerializer, CoId};
	use futures::{join, FutureExt, StreamExt};
	use libipld::Cid;
	use libp2p::{
		noise,
		swarm::{dial_opts::DialOpts, SwarmEvent},
		tcp, yamux, Multiaddr, PeerId, Swarm, SwarmBuilder,
	};
	use std::{collections::BTreeSet, time::Duration, vec};
	use tokio::select;

	struct Peer {
		peer_id: PeerId,
		addr: Multiaddr,
		swarm: Swarm<heads::Behaviour>,
	}
	impl Peer {
		fn new() -> Self {
			let mut swarm = SwarmBuilder::with_new_identity()
				.with_tokio()
				.with_tcp(tcp::Config::default(), noise::Config::new, yamux::Config::default)
				.unwrap()
				.with_behaviour(|k| Ok(heads::Behaviour::new(k.clone())))
				.unwrap()
				.with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(20)))
				.build();
			swarm.listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap()).unwrap();
			while swarm.next().now_or_never().is_some() {}
			let addr = Swarm::listeners(&swarm).next().unwrap().clone();
			Self { peer_id: swarm.local_peer_id().clone(), addr, swarm }
		}

		fn add_address(&mut self, co: CoId, peer: &Peer) {
			self.swarm
				.dial(
					DialOpts::peer_id(peer.peer_id.clone())
						.addresses(vec![peer.addr.clone()])
						.build(),
				)
				.unwrap();
			self.swarm.behaviour_mut().add_explicit_peer(co, peer.peer_id.clone());
		}

		fn swarm(&mut self) -> &mut Swarm<heads::Behaviour> {
			&mut self.swarm
		}

		async fn next(&mut self) -> Option<heads::Event> {
			loop {
				let ev = self.swarm.next().await?;
				if let SwarmEvent::Behaviour(event) = ev {
					return Some(event);
				}
			}
		}
	}

	#[tokio::test]
	async fn smoke() {
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

		// peer1: subscribe
		peer1.swarm().behaviour_mut().subscribe(&co).unwrap();

		// peer2: subscribe
		peer2.swarm().behaviour_mut().subscribe(&co).unwrap();

		// wait until both are subscribed
		let (subscribe1, subscribe2) = join!(peer1.next(), peer2.next());
		match subscribe1 {
			Some(heads::Event::Subscribed { co: event_co, peer_id }) if co == event_co && peer_id == peer2.peer_id => {
			},
			event => panic!("unexpected event: {:?}", event),
		}
		match subscribe2 {
			Some(heads::Event::Subscribed { co: event_co, peer_id }) if co == event_co && peer_id == peer1.peer_id => {
			},
			event => panic!("unexpected event: {:?}", event),
		}

		// peer2: update heads
		peer2.swarm().behaviour_mut().publish_heads(&co, h2.clone()).unwrap();

		// run
		// note: we also neeed to run peer1 to advance its state
		select! {
			event = peer1.next() => {
				match event {
					Some(heads::Event::ReceivedHeads { co: event_co, heads: event_heads, peer_id: _, response: _ }) if co == event_co && h2 == event_heads => {},
					event => panic!("unexpected event: {:?}", event),
				}
			},
			event = peer2.next() => {
				panic!("expected message event for peer1 got {:?}", event);
			},
		};
	}
}
