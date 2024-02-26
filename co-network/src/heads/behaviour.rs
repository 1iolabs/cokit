use crate::{didcomm, heads::message::HeadsMessage};
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
	collections::BTreeSet,
	task::{Context, Poll},
};

#[derive(Debug)]
pub enum Event {
	/// Receviced new heads from some peer.
	ReceivedHeads { co: CoId, heads: BTreeSet<Cid> },

	/// Subscribed for heads.
	Subscribed { co: CoId },

	/// Unsubscribed for heads.
	Unsubscribed { co: CoId },

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
		Self { inner: InnerBehaviour { didcomm: didcomm_behaviour, gossipsub: gossipsub_behaviour } }
	}

	/// Subscribe to CO gossip.
	/// Returns `true` if a new subscription has been made, `false` is was already subscribed.
	pub fn subscribe(&mut self, co: &CoId) -> Result<bool, anyhow::Error> {
		Ok(self.inner.gossipsub.subscribe(&to_topic(co))?)
	}

	pub fn unsubscribe(&mut self, co: &CoId) -> Result<bool, anyhow::Error> {
		Ok(self.inner.gossipsub.unsubscribe(&to_topic(co))?)
	}

	pub fn publish_heads(&mut self, co: &CoId, heads: BTreeSet<Cid>) -> Result<bool, anyhow::Error> {
		let message = HeadsMessage::Heads(heads);
		let data = serde_ipld_dagcbor::to_vec(&message)?;
		match self.inner.gossipsub.publish(to_topic(co), data) {
			Ok(_) => Ok(true),
			Err(PublishError::InsufficientPeers) => Ok(false),
			Err(e) => Err(e.into()),
		}
	}

	pub fn send_heads(
		&mut self,
		_co: &CoId,
		_heads: &BTreeSet<PeerId>,
		_peers: impl Iterator<Item = PeerId>,
	) -> Result<(), anyhow::Error> {
		todo!()
	}

	pub fn request_heads(&mut self, _co: &CoId, _peers: impl Iterator<Item = PeerId>) -> Result<(), anyhow::Error> {
		todo!()
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
			Poll::Ready(event) => Poll::Ready(event.map_out(|event| event.into())),
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

fn to_topic(id: &CoId) -> gossipsub::IdentTopic {
	IdentTopic::new(AsRef::<str>::as_ref(id))
}
