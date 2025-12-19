use super::{handler, inbound, protocol::MessageProtocol, EncodedMessage};
use co_identity::{IdentityResolverBox, Message, PrivateIdentityResolverBox};
use futures::{future::BoxFuture, stream::FuturesUnordered, FutureExt, StreamExt};
use libp2p::{
	core::{transport::PortUse, ConnectedPoint, Endpoint},
	swarm::{
		behaviour::{AddressChange, ConnectionClosed, DialFailure, FromSwarm},
		derive_prelude::{ConnectionEstablished, ConnectionId},
		dial_opts::DialOpts,
		ConnectionDenied, NetworkBehaviour, NotifyHandler, THandler, THandlerInEvent, THandlerOutEvent, ToSwarm,
	},
	Multiaddr, PeerId,
};
use smallvec::SmallVec;
use std::{
	collections::{HashMap, VecDeque},
	task::{Context, Poll},
};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Event {
	Received { peer_id: PeerId, message: Message },
	Sent { peer_id: PeerId, message: EncodedMessage },
	InboundFailure { peer_id: PeerId, error: String, message: Option<EncodedMessage> },
	OutboundFailure { peer_id: PeerId, error: OutboundFailure, message: Option<EncodedMessage> },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OutboundFailure {
	/// Connection timeout.
	#[error("Connection timeout")]
	Timeout,
	/// The message could not be sent because a dialing attempt failed.
	#[error("The message could not be sent because a dialing attempt failed")]
	DialFailure,
	/// The remote supports none of the requested protocols.
	#[error("The remote supports none of the requested protocols")]
	UnsupportedProtocols,
}

pub struct Config {
	/// Try to dail peer if not connected yet.
	pub auto_dail: bool,
}
impl Default for Config {
	fn default() -> Self {
		Self { auto_dail: true }
	}
}

pub struct Behaviour {
	/// Pending events to return from `poll`.
	pending_events: VecDeque<ToSwarm<Event, MessageProtocol>>,
	/// The currently connected peers, their pending outbound and inbound responses and their known,
	/// reachable addresses, if any.
	connected: HashMap<PeerId, SmallVec<[Connection; 2]>>,
	/// Requests that have not yet been sent and are waiting for a connection
	/// to be established.
	pending_outbound: HashMap<PeerId, SmallVec<[MessageProtocol; 10]>>,
	/// Config.
	config: Config,
	/// Identity resolver.
	/// Used to verify signatures of incomming messages.
	identity_resolver: IdentityResolverBox,
	/// Identites which receive encrypted messages.
	private_identity_resolver: PrivateIdentityResolverBox,
	/// Pending inbound messages.
	pending_inbound: FuturesUnordered<BoxFuture<'static, Option<Event>>>,
}
impl Behaviour {
	pub fn new(
		identity_resolver: IdentityResolverBox,
		private_identity_resolver: PrivateIdentityResolverBox,
		config: Config,
	) -> Self {
		Self {
			identity_resolver,
			pending_events: VecDeque::new(),
			connected: HashMap::new(),
			pending_outbound: HashMap::new(),
			config,
			pending_inbound: Default::default(),
			private_identity_resolver,
		}
	}

	/// Send a encoded message to peer.
	pub fn send(&mut self, peer: &PeerId, message: EncodedMessage) {
		let protocol = MessageProtocol::outbound(message);
		if let Some(protocol) = self.try_send(peer, protocol) {
			tracing::trace!(?peer, "didcomm-pending-outbound");
			if self.config.auto_dail {
				self.pending_events
					.push_back(ToSwarm::Dial { opts: DialOpts::peer_id(*peer).build() });
			}
			self.pending_outbound.entry(*peer).or_default().push(protocol);
		}
	}
}
impl Behaviour {
	/// Tries to send a message by queueing an appropriate event to be
	/// emitted to the `Swarm`. If the peer is not currently connected,
	/// the given request is return unchanged.
	fn try_send(&mut self, peer: &PeerId, protocol: MessageProtocol) -> Option<MessageProtocol> {
		if let Some(connections) = self.connected.get_mut(peer) {
			if connections.is_empty() {
				return Some(protocol);
			}
			// let ix = (request.request_id.0 as usize) % connections.len();
			let conn = &mut connections[0]; // TODO: choose random?
								   // conn.pending_inbound_responses.insert(request.request_id);
								   // tracing::trace!(?peer, connection_id = ?conn.id, "try-send");
			self.pending_events.push_back(ToSwarm::NotifyHandler {
				peer_id: *peer,
				handler: NotifyHandler::One(conn.id),
				event: protocol,
			});
			None
		} else {
			Some(protocol)
		}
	}

	fn on_connection_established(
		&mut self,
		ConnectionEstablished { peer_id, connection_id, endpoint, other_established, .. }: ConnectionEstablished,
	) {
		let address = match endpoint {
			ConnectedPoint::Dialer { address, .. } => Some(address.clone()),
			ConnectedPoint::Listener { .. } => None,
		};
		self.connected
			.entry(peer_id)
			.or_default()
			.push(Connection::new(connection_id, address));

		if other_established == 0 {
			if let Some(pending) = self.pending_outbound.remove(&peer_id) {
				for protocol in pending {
					let request = self.try_send(&peer_id, protocol);
					assert!(request.is_none());
				}
			}
		}
	}

	fn on_connection_closed(
		&mut self,
		ConnectionClosed { peer_id, connection_id, remaining_established, .. }: ConnectionClosed<'_>,
	) {
		let connections = self
			.connected
			.get_mut(&peer_id)
			.expect("Expected some established connection to peer before closing.");

		let _connection = connections
			.iter()
			.position(|c| c.id == connection_id)
			.map(|p: usize| connections.remove(p))
			.expect("Expected connection to be established before closing.");

		debug_assert_eq!(connections.is_empty(), remaining_established == 0);
		if connections.is_empty() {
			self.connected.remove(&peer_id);
		}
	}

	fn on_dial_failure(&mut self, DialFailure { peer_id, connection_id, error }: DialFailure) {
		if let Some(peer_id) = peer_id {
			// If there are pending outgoing requests when a dial failure occurs,
			// it is implied that we are not connected to the peer, since pending
			// outgoing requests are drained when a connection is established and
			// only created when a peer is not connected when a request is made.
			// Thus these requests must be considered failed, even if there is
			// another, concurrent dialing attempt ongoing.
			if let Some(pending) = self.pending_outbound.remove(&peer_id) {
				tracing::warn!(?peer_id, ?connection_id, ?error, message_discard_count = pending.len(), "dail-failure");
				for request in pending {
					if let Some(message) = request.into_message() {
						self.pending_events.push_back(ToSwarm::GenerateEvent(Event::OutboundFailure {
							peer_id,
							message: Some(message),
							error: OutboundFailure::DialFailure,
						}));
					}
				}
			}
		}
	}

	fn on_address_change(&mut self, AddressChange { peer_id, connection_id, new, .. }: AddressChange) {
		let new_address = match new {
			ConnectedPoint::Dialer { address, .. } => Some(address.clone()),
			ConnectedPoint::Listener { .. } => None,
		};
		let connections = self
			.connected
			.get_mut(&peer_id)
			.expect("Address change can only happen on an established connection.");

		let connection = connections
			.iter_mut()
			.find(|c| c.id == connection_id)
			.expect("Address change can only happen on an established connection.");
		connection.address = new_address;
	}
}
impl NetworkBehaviour for Behaviour {
	type ConnectionHandler = handler::Handler;
	type ToSwarm = Event;

	fn poll(&mut self, cx: &mut Context<'_>) -> Poll<ToSwarm<Self::ToSwarm, THandlerInEvent<Self>>> {
		// events
		if let Some(event) = self.pending_events.pop_front() {
			return Poll::Ready(event);
		} else if self.pending_events.capacity() > 100 {
			self.pending_events.shrink_to_fit();
		}

		// pending inbound
		if let Poll::Ready(Some(Some(event))) = self.pending_inbound.poll_next_unpin(cx) {
			return Poll::Ready(ToSwarm::GenerateEvent(event));
		}

		// pending
		Poll::Pending
	}

	fn handle_established_inbound_connection(
		&mut self,
		_connection_id: ConnectionId,
		_peer: PeerId,
		_local_addr: &Multiaddr,
		_remote_addr: &Multiaddr,
	) -> Result<THandler<Self>, ConnectionDenied> {
		Ok(handler::Handler::new())
	}

	fn handle_established_outbound_connection(
		&mut self,
		_connection_id: ConnectionId,
		_peer: PeerId,
		_addr: &Multiaddr,
		_role_override: Endpoint,
		_port_use: PortUse,
	) -> Result<THandler<Self>, ConnectionDenied> {
		Ok(handler::Handler::new())
	}

	fn on_swarm_event(&mut self, event: FromSwarm) {
		match event {
			FromSwarm::ConnectionEstablished(connection_established) => {
				self.on_connection_established(connection_established)
			},
			FromSwarm::ConnectionClosed(connection_closed) => self.on_connection_closed(connection_closed),
			FromSwarm::AddressChange(address_change) => self.on_address_change(address_change),
			FromSwarm::DialFailure(dial_failure) => self.on_dial_failure(dial_failure),
			_ => {},
		}
	}

	fn on_connection_handler_event(
		&mut self,
		peer: PeerId,
		_connection_id: ConnectionId,
		event: THandlerOutEvent<Self>,
	) {
		// tracing::debug!(?peer, ?event, "on_connection_handler_event");
		match event {
			handler::Event::Received { message } => {
				self.pending_inbound.push(
					inbound::inbound_message(
						self.identity_resolver.clone(),
						self.private_identity_resolver.clone(),
						peer,
						message,
					)
					.boxed(),
				);
			},
			handler::Event::Sent { message } => {
				self.pending_events
					.push_back(ToSwarm::GenerateEvent(Event::Sent { peer_id: peer, message }));
			},
			handler::Event::OutboundUnsupportedProtocols => {
				self.pending_events.push_back(ToSwarm::GenerateEvent(Event::OutboundFailure {
					peer_id: peer,
					message: None,
					error: OutboundFailure::UnsupportedProtocols,
				}));
			},
			handler::Event::OutboundTimeout => {
				self.pending_events.push_back(ToSwarm::GenerateEvent(Event::OutboundFailure {
					peer_id: peer,
					message: None,
					error: OutboundFailure::Timeout,
				}));
			},
		}
	}
}

struct Connection {
	id: ConnectionId,
	address: Option<Multiaddr>,
}

impl Connection {
	fn new(connection_id: ConnectionId, address: Option<Multiaddr>) -> Connection {
		Connection { id: connection_id, address }
	}
}

#[cfg(test)]
mod tests {
	use crate::didcomm;
	use co_identity::{
		DidCommHeader, DidKeyIdentity, DidKeyIdentityResolver, IdentityResolver, IdentityResolverBox,
		MemoryPrivateIdentityResolver, Message, PrivateIdentity, PrivateIdentityResolver, PrivateIdentityResolverBox,
	};
	use futures::{join, FutureExt, StreamExt};
	use libp2p::{
		noise,
		swarm::{dial_opts::DialOpts, SwarmEvent},
		tcp, yamux, Multiaddr, PeerId, Swarm, SwarmBuilder,
	};
	use std::{time::Duration, vec};

	struct Peer {
		peer_id: PeerId,
		addr: Multiaddr,
		swarm: Swarm<didcomm::Behaviour>,
	}
	impl Peer {
		fn new(resolver: IdentityResolverBox, private_resolver: PrivateIdentityResolverBox) -> Self {
			let mut swarm = SwarmBuilder::with_new_identity()
				.with_tokio()
				.with_tcp(tcp::Config::default(), noise::Config::new, yamux::Config::default)
				.unwrap()
				.with_behaviour(|_keypair| {
					Ok(didcomm::Behaviour::new(resolver, private_resolver, didcomm::Config { auto_dail: false }))
				})
				.unwrap()
				.with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(20)))
				.build();
			swarm.listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap()).unwrap();
			while swarm.next().now_or_never().is_some() {}
			let addr = Swarm::listeners(&swarm).next().unwrap().clone();
			Self { peer_id: *swarm.local_peer_id(), addr, swarm }
		}

		fn peer_id(&self) -> PeerId {
			self.peer_id
		}

		fn add_address(&mut self, peer: &Peer) {
			self.swarm
				.dial(DialOpts::peer_id(peer.peer_id).addresses(vec![peer.addr.clone()]).build())
				.unwrap();
			// when we dail just the peerid we always get an dail error because we have no addresses
			// self.swarm.behaviour_mut().add_explicit_peer(co, peer.peer_id.clone());
		}

		fn swarm(&mut self) -> &mut Swarm<didcomm::Behaviour> {
			&mut self.swarm
		}
	}

	async fn send_and_recv(peer1: &mut Peer, peer2: &mut Peer) {
		let peer_id1 = peer1.peer_id();
		let peer_id2 = peer2.peer_id();

		// send
		let send_body = "Hello";
		let send_header = DidCommHeader::new("test");
		let (_send_message_id, send_message) =
			didcomm::EncodedMessage::create_plain_json(send_header.clone(), &send_body).unwrap();
		peer1.swarm().behaviour_mut().send(&peer_id2, send_message.clone());

		// wait sent
		join!(
			async {
				loop {
					if let Some(SwarmEvent::Behaviour(event)) = peer1.swarm().next().await {
						match event {
							didcomm::Event::Sent { peer_id, message } => {
								assert_eq!(peer_id, peer_id2);
								assert_eq!(message, send_message);
							},
							e => panic!("peer1: invalid event: {:?}", e),
						}
						break;
					}
				}
			},
			async {
				loop {
					if let Some(SwarmEvent::Behaviour(event)) = peer2.swarm().next().await {
						match event {
							didcomm::Event::Received { peer_id, message } => {
								assert_eq!(peer_id, peer_id1);
								if let Message::PlainJson { header, body } = message {
									assert_eq!(header, send_header);
									assert_eq!(serde_json::from_str::<&str>(&body).unwrap(), send_body);
								} else {
									panic!("peer2: invalid message: {:?}", message);
								}
							},
							e => panic!("peer2: invalid event: {:?}", e),
						}
						break;
					}
				}
			},
		);
	}

	#[tokio::test]
	async fn test_smoke() {
		// identities
		let identity1 = DidKeyIdentity::generate(Some(&[1; 32]));
		let identity2 = DidKeyIdentity::generate(Some(&[2; 32]));

		// peers
		let mut peer1 = Peer::new(
			DidKeyIdentityResolver::new().boxed(),
			MemoryPrivateIdentityResolver::from([identity1.clone().boxed()]).boxed(),
		);
		let mut peer2 = Peer::new(
			DidKeyIdentityResolver::new().boxed(),
			MemoryPrivateIdentityResolver::from([identity2.clone().boxed()]).boxed(),
		);
		peer2.add_address(&peer1);

		// test
		for _ in 0..1000 {
			send_and_recv(&mut peer1, &mut peer2).await;
		}
	}
}
