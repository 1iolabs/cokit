use std::{task::{Context, Poll}, collections::VecDeque};

use libp2p::swarm::{ConnectionHandler, ConnectionHandlerEvent, SubstreamProtocol, KeepAlive, handler::{ConnectionEvent, FullyNegotiatedInbound, FullyNegotiatedOutbound, DialUpgradeError, ListenUpgradeError}, StreamUpgradeError};

use super::{protocol::MessageProtocol, codec, message::Message};

#[derive(Debug, Clone)]
pub enum Event
{
    Sent { message: Message },
    Received { message: Message },
    OutboundUnsupportedProtocols,
    OutboundTimeout,
}

pub struct Handler {
    keep_alive: KeepAlive,

    /// A pending fatal error that results in the connection being closed.
    pending_error: Option<codec::Error>,
    
    /// Pending events to be emitted by `poll`.
    pending_events: VecDeque<Event>,
 
    /// Outbound messages to be sent.
    outbound: VecDeque<MessageProtocol>,
}

impl Handler {
    pub fn new() -> Self {
        Handler {
            keep_alive: KeepAlive::Yes,
            outbound: VecDeque::new(),
            pending_error: None,
            pending_events: VecDeque::new(),
        }
    }
}

impl Handler {
    fn on_dial_upgrade_error(
        &mut self,
        DialUpgradeError { error, .. }: DialUpgradeError<
            <Self as ConnectionHandler>::OutboundOpenInfo,
            <Self as ConnectionHandler>::OutboundProtocol,
        >,
    ) {
        match error {
            StreamUpgradeError::Timeout => {
                self.pending_events.push_back(Event::OutboundTimeout);
            }
            StreamUpgradeError::NegotiationFailed => {
                // The remote merely doesn't support the protocol(s) we requested.
                // This is no reason to close the connection, which may
                // successfully communicate with other protocols already.
                // An event is reported to permit user code to react to the fact that
                // the remote peer does not support the requested protocol(s).
                self.pending_events.push_back(Event::OutboundUnsupportedProtocols);
            }
            StreamUpgradeError::Apply(_e) => {
                // log::debug!("outbound stream {info} failed: {e}");
            }
            StreamUpgradeError::Io(_e) => {
                // log::debug!("outbound stream {info} failed: {e}");
            }
        }
    }
    fn on_listen_upgrade_error(
        &mut self,
        ListenUpgradeError { .. }: ListenUpgradeError<
            <Self as ConnectionHandler>::InboundOpenInfo,
            <Self as ConnectionHandler>::InboundProtocol,
        >,
    ) {
        // log::debug!("inbound stream {info} failed: {error}");
    }
}

impl ConnectionHandler for Handler {
    type FromBehaviour = MessageProtocol;
    type ToBehaviour = Event;
    type Error = codec::Error;
    type InboundProtocol = MessageProtocol;
    type OutboundProtocol = MessageProtocol;
    type OutboundOpenInfo = ();
    type InboundOpenInfo = ();

    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundProtocol, ()> {
        SubstreamProtocol::new(MessageProtocol::inbound(), ())
    }

    fn on_behaviour_event(&mut self, v: Self::FromBehaviour) {
        self.outbound.push_back(v);
    }

    fn connection_keep_alive(&self) -> KeepAlive {
        self.keep_alive
    }

    fn poll(
        &mut self,
        _cx: &mut Context<'_>,
    ) -> Poll<ConnectionHandlerEvent<Self::OutboundProtocol, Self::OutboundOpenInfo, Self::ToBehaviour, Self::Error>>
    {
        // check for a pending (fatal) error
        if let Some(err) = self.pending_error.take() {
            // The handler will not be polled again by the `Swarm`.
            return Poll::Ready(ConnectionHandlerEvent::Close(err));
        }

        // drain pending events
        if let Some(event) = self.pending_events.pop_front() {
            return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(event));
        } else if self.pending_events.capacity() > 100 {
            self.pending_events.shrink_to_fit();
        }

        // open outbound streams
        if let Some(message) = self.outbound.pop_front() {
            return Poll::Ready(ConnectionHandlerEvent::OutboundSubstreamRequest {
                protocol: SubstreamProtocol::new(message, ())
            });
        } else if self.outbound.capacity() > 100 {
            self.outbound.shrink_to_fit();
        }

        // nothing todo right now
        Poll::Pending
    }

    fn on_connection_event(
        &mut self,
        event: ConnectionEvent<
            Self::InboundProtocol,
            Self::OutboundProtocol,
            Self::InboundOpenInfo,
            Self::OutboundOpenInfo,
        >,
    ) {
        match event {
            ConnectionEvent::FullyNegotiatedInbound(FullyNegotiatedInbound {protocol, .. }) => {
                self.pending_events.push_back(Event::Received { message: protocol });
            },
            ConnectionEvent::FullyNegotiatedOutbound(FullyNegotiatedOutbound { protocol, .. }) => {
                if let Some(message) = protocol {
                    self.pending_events.push_back(Event::Sent { message });
                }
            },
            ConnectionEvent::DialUpgradeError(dial_upgrade_error) => self.on_dial_upgrade_error(dial_upgrade_error),
            ConnectionEvent::ListenUpgradeError(listen_upgrade_error) => self.on_listen_upgrade_error(listen_upgrade_error),
            ConnectionEvent::AddressChange(_) => {},
            ConnectionEvent::LocalProtocolsChange(_) => {},
            ConnectionEvent::RemoteProtocolsChange(_) => {},
        }
    }
}
