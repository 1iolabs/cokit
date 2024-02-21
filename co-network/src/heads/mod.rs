use libipld::Cid;
use libp2p::{
	gossipsub::{Behaviour, Event, IdentTopic, TopicHash},
	PeerId,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
enum HeadsMessage {
	#[serde(rename = "h")]
	Heads(BTreeSet<Cid>),
}

pub trait HeadsHandler {
	/// Received heads from peers.
	/// This will executed every time when heads have received.
	/// The implementation has to choose if they will be used and call `publish_heads` if heads changed.
	fn on_heads(&mut self, heads: BTreeSet<Cid>);

	/// New peer joined.
	fn on_subscribe(&mut self, peer: PeerId);

	/// Peer left.
	fn on_unsubscribe(&mut self, peer: PeerId);
}

pub struct Heads {
	topic: IdentTopic,
	hash: TopicHash,
	heads: BTreeSet<Cid>,
	// TODO: do we need to try PeerId's?
	// TODO: Can we check this using gossipsub itself?
	subscriptions: i32,
	handler: Box<dyn HeadsHandler + Send + Sync + 'static>,
}
impl Heads {
	pub fn subscribe<H: HeadsHandler + Send + Sync + 'static>(
		gossipsub: &mut Behaviour,
		topic: IdentTopic,
		handler: H,
	) -> Result<Heads, anyhow::Error> {
		gossipsub.subscribe(&topic)?;
		Ok(Heads { hash: topic.hash(), topic, handler: Box::new(handler), subscriptions: 0, heads: Default::default() })
	}

	pub fn set_heads(&mut self, gossipsub: &mut Behaviour, heads: BTreeSet<Cid>) -> Result<(), anyhow::Error> {
		// assign
		if self.heads == heads {
			return Ok(())
		}
		self.heads = heads;

		// publish
		self.try_publish(gossipsub)
	}

	fn publish(&self, gossipsub: &mut Behaviour, message: &HeadsMessage) -> Result<(), anyhow::Error> {
		tracing::info!(?self.topic, ?message, "heads-publish");
		let message = serde_ipld_dagcbor::to_vec(message)?;
		gossipsub.publish(self.topic.clone(), message).map(|_| ())?;
		Ok(())
	}

	pub fn unsubscribe(self, gossipsub: &mut Behaviour) -> Result<(), anyhow::Error> {
		gossipsub.unsubscribe(&self.topic)?;
		Ok(())
	}

	fn try_publish(&mut self, gossipsub: &mut Behaviour) -> Result<(), anyhow::Error> {
		if !self.heads.is_empty() && self.subscriptions > 0 {
			self.publish(gossipsub, &HeadsMessage::Heads(self.heads.clone()))?;
		}
		Ok(())
	}

	pub fn handle_swarm_event(&mut self, event: Event) -> Option<Event> {
		let is_our_event = match &event {
			Event::Message { propagation_source: _, message_id: _, message } => self.hash == message.topic,
			Event::Subscribed { peer_id: _, topic } => &self.hash == topic,
			Event::Unsubscribed { peer_id: _, topic } => &self.hash == topic,
			Event::GossipsubNotSupported { peer_id: _ } => false,
		};
		if is_our_event {
			match event {
				Event::Message { propagation_source: _, message_id: _, message } => {
					let heads_message: HeadsMessage = match serde_ipld_dagcbor::from_slice(&message.data) {
						Ok(m) => m,
						Err(err) => {
							tracing::warn!(?err, ?self.topic, "received-invalid-message");
							return None;
						},
					};
					match heads_message {
						HeadsMessage::Heads(heads) => {
							self.handler.on_heads(heads);
						},
					}
				},
				Event::Subscribed { peer_id, topic: _ } => {
					self.subscriptions += 1;
					self.handler.on_subscribe(peer_id);
				},
				Event::Unsubscribed { peer_id, topic: _ } => {
					self.subscriptions -= 1;
					self.handler.on_unsubscribe(peer_id);
				},
				_ => {},
			}
			None
		} else {
			Some(event)
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::{Heads, HeadsHandler};
	use co_primitives::BlockSerializer;
	use futures::{join, FutureExt, StreamExt};
	use libipld::{Block, Cid, DefaultParams};
	use libp2p::{
		gossipsub::{Behaviour, Event, IdentTopic, MessageAuthenticity},
		noise,
		swarm::{dial_opts::DialOpts, SwarmEvent},
		tcp, yamux, Multiaddr, PeerId, Swarm, SwarmBuilder,
	};
	use std::{
		collections::{BTreeMap, BTreeSet},
		str::from_utf8,
		time::Duration,
		vec,
	};
	use tokio::{
		select,
		sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
	};

	struct Peer {
		name: String,
		peer_id: PeerId,
		addr: Multiaddr,
		swarm: Swarm<Behaviour>,
	}
	impl Peer {
		fn new(name: &str) -> Self {
			let mut swarm = SwarmBuilder::with_new_identity()
				.with_tokio()
				.with_tcp(tcp::Config::default(), noise::Config::new, yamux::Config::default)
				.unwrap()
				.with_behaviour(|k| Ok(Behaviour::new(MessageAuthenticity::Signed(k.clone()), Default::default())?))
				.unwrap()
				.with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(20)))
				.build();
			swarm.listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap()).unwrap();
			while swarm.next().now_or_never().is_some() {}
			let addr = Swarm::listeners(&swarm).next().unwrap().clone();
			Self { name: name.to_owned(), peer_id: swarm.local_peer_id().clone(), addr, swarm }
		}

		fn add_address(&mut self, peer: &Peer) {
			self.swarm
				.dial(
					DialOpts::peer_id(peer.peer_id.clone())
						.addresses(vec![peer.addr.clone()])
						.build(),
				)
				.unwrap();
			self.swarm.behaviour_mut().add_explicit_peer(&peer.peer_id);
		}

		fn swarm(&mut self) -> &mut Swarm<Behaviour> {
			&mut self.swarm
		}

		async fn next(&mut self) -> Option<Event> {
			loop {
				let ev = self.swarm.next().await?;
				if let SwarmEvent::Behaviour(event) = ev {
					return Some(event);
				}
			}
		}

		async fn run_once(&mut self, heads: &mut Heads) {
			let event = self.next().await.unwrap();
			tracing::debug!(peer_name = self.name, ?event, "event");
			assert!(heads.handle_swarm_event(event).is_none());
		}
	}

	#[derive(Debug, Clone, PartialEq)]
	enum HandlerEvent {
		Heads(BTreeSet<Cid>),
		Subscribe(PeerId),
		Unsubscribe(PeerId),
	}
	struct Handler {
		tx: UnboundedSender<HandlerEvent>,
	}
	impl Handler {
		pub fn new() -> (Self, UnboundedReceiver<HandlerEvent>) {
			let (tx, rx) = unbounded_channel();
			(Self { tx }, rx)
		}
	}
	impl HeadsHandler for Handler {
		fn on_heads(&mut self, heads: BTreeSet<Cid>) {
			self.tx.send(HandlerEvent::Heads(heads)).unwrap();
		}

		fn on_subscribe(&mut self, peer: PeerId) {
			self.tx.send(HandlerEvent::Subscribe(peer)).unwrap();
		}

		fn on_unsubscribe(&mut self, peer: PeerId) {
			self.tx.send(HandlerEvent::Unsubscribe(peer)).unwrap();
		}
	}

	#[tokio::test]
	async fn smoke() {
		let topic = IdentTopic::new("test");

		// test data
		let test = BlockSerializer::default().serialize(&"test").unwrap();
		let hello = BlockSerializer::default().serialize(&"hello").unwrap();

		// heads
		let mut h1: BTreeSet<Cid> = BTreeSet::new();
		h1.insert(test.cid().clone());
		let mut h2: BTreeSet<Cid> = BTreeSet::new();
		h2.insert(hello.cid().clone());

		// peers
		let mut peer1 = Peer::new("peer1");
		let mut peer2 = Peer::new("peer2");
		peer2.add_address(&peer1);

		// peer1: subscribe
		let (handler1, mut receiver1) = Handler::new();
		let mut heads1 = Heads::subscribe(peer1.swarm().behaviour_mut(), topic.clone(), handler1).unwrap();

		// peer2: subscribe
		let (handler2, mut receiver2) = Handler::new();
		let mut heads2 = Heads::subscribe(peer2.swarm().behaviour_mut(), topic.clone(), handler2).unwrap();

		// wait until both are subscribed
		join!(peer1.run_once(&mut heads1), peer2.run_once(&mut heads2));
		assert_eq!(Some(HandlerEvent::Subscribe(peer2.peer_id.clone())), receiver1.recv().await);
		assert_eq!(Some(HandlerEvent::Subscribe(peer1.peer_id.clone())), receiver2.recv().await);

		// peer2: update heads
		heads2.set_heads(peer2.swarm().behaviour_mut(), h2.clone()).unwrap();

		// run
		// note: we also neeed to run peer1 to advance its state
		select! {
			_ = peer2.run_once(&mut heads2) => {},
			_ = peer1.run_once(&mut heads1) => {},
		};

		// peer1: wait for heads
		assert_eq!(Some(HandlerEvent::Heads(h2.clone())), receiver1.recv().await);
	}

	// #[tokio::test]
	async fn _unfinished_test_different_heads() {
		let topic = IdentTopic::new("test");

		// test data
		let test = BlockSerializer::default().serialize(&"test").unwrap();
		let hello = BlockSerializer::default().serialize(&"hello").unwrap();
		let test_hello = BlockSerializer::default().serialize(&"test hello").unwrap();
		let mut blocks: BTreeMap<Cid, Block<DefaultParams>> = BTreeMap::new();
		blocks.insert(test.cid().clone(), test.clone());
		blocks.insert(hello.cid().clone(), hello.clone());
		blocks.insert(test_hello.cid().clone(), test_hello.clone());
		fn merge(blocks: &BTreeMap<Cid, Block<DefaultParams>>, a: &BTreeSet<Cid>, b: &BTreeSet<Cid>) -> BTreeSet<Cid> {
			let vec: Vec<&str> = a
				.iter()
				.chain(b)
				.map(|cid| from_utf8(blocks.get(cid).unwrap().data()).unwrap())
				.collect();
			let merged = vec.join(" ");
			let merged_cid = BlockSerializer::default().serialize(&merged).unwrap().cid().clone();
			let mut heads: BTreeSet<Cid> = BTreeSet::new();
			heads.insert(merged_cid);
			heads
		}

		// loop {
		// 	select! {
		// 		event1 = peer1.next() => {
		// 			println!("event1: {:?}", event1);
		// 		},
		// 		event2 = peer2.next() => {
		// 			println!("event2: {:?}", event2);
		// 		},
		// 	}
		// }

		// heads
		let mut h1: BTreeSet<Cid> = BTreeSet::new();
		h1.insert(test.cid().clone());
		let mut h2: BTreeSet<Cid> = BTreeSet::new();
		h2.insert(hello.cid().clone());
		let mut _h3: BTreeSet<Cid> = BTreeSet::new();
		h2.insert(test_hello.cid().clone());

		// peers
		let mut peer1 = Peer::new("peer1");
		let mut peer2 = Peer::new("peer2");
		peer2.add_address(&peer1);

		// peer1: subscribe
		let (handler1, mut receiver1) = Handler::new();
		let mut heads1 = Heads::subscribe(peer1.swarm().behaviour_mut(), topic.clone(), handler1).unwrap();

		// peer2: subscribe
		let (handler2, mut receiver2) = Handler::new();
		let mut heads2 = Heads::subscribe(peer2.swarm().behaviour_mut(), topic.clone(), handler2).unwrap();

		// wait until both are subscribed
		join!(peer1.run_once(&mut heads1), peer2.run_once(&mut heads2));
		assert_eq!(Some(HandlerEvent::Subscribe(peer2.peer_id.clone())), receiver1.recv().await);
		assert_eq!(Some(HandlerEvent::Subscribe(peer1.peer_id.clone())), receiver2.recv().await);

		// peer2: update heads
		heads2.set_heads(peer2.swarm().behaviour_mut(), h2.clone()).unwrap();

		// run
		// note: we also neeed to run peer1 to advance its state
		select! {
			_ = peer2.run_once(&mut heads2) => {},
			_ = peer1.run_once(&mut heads1) => {},
		};

		// peer1: wait for heads
		assert_eq!(Some(HandlerEvent::Heads(h2.clone())), receiver1.recv().await);
	}
}
