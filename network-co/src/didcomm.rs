use std::{
	collections::{HashMap, VecDeque},
	task::{Context, Poll},
};

use libp2p::{
	core::{ConnectedPoint, Endpoint},
	swarm::{
		derive_prelude::{ConnectionEstablished, ConnectionId},
		dial_opts::DialOpts,
		AddressChange, ConnectionClosed, ConnectionDenied, DialFailure, FromSwarm, NetworkBehaviour, NotifyHandler,
		PollParameters, THandler, THandlerInEvent, THandlerOutEvent, ToSwarm,
	},
	Multiaddr, PeerId,
};
use smallvec::SmallVec;

use self::{handler::Handler, protocol::MessageProtocol};

mod codec;
mod handler;
mod message;
mod protocol;

pub use message::Message;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
	Received { peer_id: PeerId, message: Message },
	Sent { peer_id: PeerId, message: Message },
	OutboundFailure { peer_id: PeerId, error: OutboundFailure, message: Option<Message> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutboundFailure {
	/// Conenction timeout.
	Timeout,
	/// The message could not be sent because a dialing attempt failed.
	DialFailure,
	/// The remote supports none of the requested protocols.
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

pub struct Behavior {
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
}

impl Behavior {
	pub fn new(config: Config) -> Self {
		Self { pending_events: VecDeque::new(), connected: HashMap::new(), pending_outbound: HashMap::new(), config }
	}

	pub fn send(&mut self, peer: &PeerId, message: Message) {
		let protocol = MessageProtocol::outbound(message);
		if let Some(protocol) = self.try_send(peer, protocol) {
			if self.config.auto_dail {
				self.pending_events
					.push_back(ToSwarm::Dial { opts: DialOpts::peer_id(*peer).build() });
			}
			self.pending_outbound.entry(*peer).or_default().push(protocol);
		}
	}
}

impl Behavior {
	/// Tries to send a message by queueing an appropriate event to be
	/// emitted to the `Swarm`. If the peer is not currently connected,
	/// the given request is return unchanged.
	fn try_send(&mut self, peer: &PeerId, protocol: MessageProtocol) -> Option<MessageProtocol> {
		if let Some(connections) = self.connected.get_mut(peer) {
			if connections.is_empty() {
				return Some(protocol)
			}
			// let ix = (request.request_id.0 as usize) % connections.len();
			let conn = &mut connections[0]; // TODO: choose random?
								// conn.pending_inbound_responses.insert(request.request_id);
			tracing::info!(?peer, connection_id = ?conn.id, "try-send");
			self.pending_events.push_back(ToSwarm::NotifyHandler {
				peer_id: *peer,
				handler: NotifyHandler::One(conn.id),
				event: protocol,
			});
			None
		} else {
			tracing::info!(?peer, deferred = true, "try-send");
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
		ConnectionClosed { peer_id, connection_id, remaining_established, .. }: ConnectionClosed<
			<Self as NetworkBehaviour>::ConnectionHandler,
		>,
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
			// log
			let message_discard_count = match self.pending_outbound.get(&peer_id) {
				Some(v) => v.len(),
				None => 0,
			};
			if message_discard_count > 0 {
				tracing::warn!(?peer_id, ?connection_id, ?error, ?message_discard_count, "dail-failure");
			} else {
				tracing::info!(?peer_id, ?connection_id, ?error, ?message_discard_count, "dail-failure");
			}

			// If there are pending outgoing requests when a dial failure occurs,
			// it is implied that we are not connected to the peer, since pending
			// outgoing requests are drained when a connection is established and
			// only created when a peer is not connected when a request is made.
			// Thus these requests must be considered failed, even if there is
			// another, concurrent dialing attempt ongoing.
			if let Some(pending) = self.pending_outbound.remove(&peer_id) {
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

impl Default for Behavior {
	fn default() -> Self {
		Self::new(Default::default())
	}
}

impl NetworkBehaviour for Behavior {
	type ConnectionHandler = Handler;
	type ToSwarm = Event;

	fn poll(
		&mut self,
		_cx: &mut Context<'_>,
		_params: &mut impl PollParameters,
	) -> Poll<ToSwarm<Self::ToSwarm, THandlerInEvent<Self>>> {
		if let Some(event) = self.pending_events.pop_front() {
			return Poll::Ready(event)
		} else if self.pending_events.capacity() > 100 {
			self.pending_events.shrink_to_fit();
		}

		Poll::Pending
	}

	fn handle_established_inbound_connection(
		&mut self,
		_connection_id: ConnectionId,
		_peer: PeerId,
		_local_addr: &Multiaddr,
		_remote_addr: &Multiaddr,
	) -> Result<THandler<Self>, ConnectionDenied> {
		Ok(Handler::new())
	}

	fn handle_established_outbound_connection(
		&mut self,
		_connection_id: ConnectionId,
		_peer: PeerId,
		_addr: &Multiaddr,
		_role_override: Endpoint,
	) -> Result<THandler<Self>, ConnectionDenied> {
		Ok(Handler::new())
	}

	fn on_swarm_event(&mut self, event: FromSwarm<Self::ConnectionHandler>) {
		match event {
			FromSwarm::ConnectionEstablished(connection_established) =>
				self.on_connection_established(connection_established),
			FromSwarm::ConnectionClosed(connection_closed) => self.on_connection_closed(connection_closed),
			FromSwarm::AddressChange(address_change) => self.on_address_change(address_change),
			FromSwarm::DialFailure(dial_failure) => self.on_dial_failure(dial_failure),
			FromSwarm::ListenFailure(_) => {},
			FromSwarm::NewListener(_) => {},
			FromSwarm::NewListenAddr(_) => {},
			FromSwarm::ExpiredListenAddr(_) => {},
			FromSwarm::ListenerError(_) => {},
			FromSwarm::ListenerClosed(_) => {},
			FromSwarm::NewExternalAddrCandidate(_) => {},
			FromSwarm::ExternalAddrExpired(_) => {},
			FromSwarm::ExternalAddrConfirmed(_) => {},
		}
	}

	fn on_connection_handler_event(
		&mut self,
		peer: PeerId,
		_connection_id: ConnectionId,
		event: THandlerOutEvent<Self>,
	) {
		tracing::debug!(?peer, ?event, "on_connection_handler_event");
		match event {
			handler::Event::Received { message } => {
				self.pending_events
					.push_back(ToSwarm::GenerateEvent(Event::Received { peer_id: peer, message }));
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
