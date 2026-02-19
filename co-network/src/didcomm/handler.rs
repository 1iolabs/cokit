// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use super::{message::EncodedMessage, protocol::MessageProtocol};
use libp2p::swarm::{
	handler::{ConnectionEvent, DialUpgradeError, FullyNegotiatedInbound, FullyNegotiatedOutbound, ListenUpgradeError},
	ConnectionHandler, ConnectionHandlerEvent, StreamUpgradeError, SubstreamProtocol,
};
use std::{
	collections::VecDeque,
	task::{Context, Poll},
};

#[derive(Debug, Clone)]
pub enum Event {
	Sent { message: EncodedMessage },
	Received { message: EncodedMessage },
	OutboundUnsupportedProtocols,
	OutboundTimeout,
}

pub struct Handler {
	/// Pending events to be emitted by `poll`.
	pending_events: VecDeque<Event>,

	/// Outbound messages to be sent.
	outbound: VecDeque<MessageProtocol>,
	pending_outbound: i32,
}

impl Handler {
	pub fn new() -> Self {
		Handler { outbound: VecDeque::new(), pending_events: VecDeque::new(), pending_outbound: 0 }
	}
}

impl Handler {
	fn on_dial_upgrade_error(
		&mut self,
		DialUpgradeError { error, info }: DialUpgradeError<
			<Self as ConnectionHandler>::OutboundOpenInfo,
			<Self as ConnectionHandler>::OutboundProtocol,
		>,
	) {
		match error {
			StreamUpgradeError::Timeout => {
				self.pending_events.push_back(Event::OutboundTimeout);
			},
			StreamUpgradeError::NegotiationFailed => {
				// The remote merely doesn't support the protocol(s) we requested.
				// This is no reason to close the connection, which may
				// successfully communicate with other protocols already.
				// An event is reported to permit user code to react to the fact that
				// the remote peer does not support the requested protocol(s).
				self.pending_events.push_back(Event::OutboundUnsupportedProtocols);
			},
			StreamUpgradeError::Apply(err) => {
				// log::debug!("outbound stream {info} failed: {e}");
				tracing::trace!(?err, ?info, "didcomm-outbound-stream-apply-failed");
			},
			StreamUpgradeError::Io(err) => {
				// log::debug!("outbound stream {info} failed: {e}");
				tracing::trace!(?err, ?info, "didcomm-outbound-stream-io-failed");
			},
		}
	}

	fn on_listen_upgrade_error(
		&mut self,
		ListenUpgradeError { error, info }: ListenUpgradeError<
			<Self as ConnectionHandler>::InboundOpenInfo,
			<Self as ConnectionHandler>::InboundProtocol,
		>,
	) {
		tracing::trace!(err = ?error, ?info, "didcomm-inbound-stream-failed");
		// log::debug!("inbound stream {info} failed: {error}");
	}
}

impl ConnectionHandler for Handler {
	type FromBehaviour = MessageProtocol;
	type ToBehaviour = Event;
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

	fn connection_keep_alive(&self) -> bool {
		self.pending_outbound > 0 || !self.outbound.is_empty() || !self.pending_events.is_empty()
	}

	fn poll(
		&mut self,
		_cx: &mut Context<'_>,
	) -> Poll<ConnectionHandlerEvent<Self::OutboundProtocol, Self::OutboundOpenInfo, Self::ToBehaviour>> {
		// drain pending events
		if let Some(event) = self.pending_events.pop_front() {
			return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(event));
		} else if self.pending_events.capacity() > 100 {
			self.pending_events.shrink_to_fit();
		}

		// open outbound streams
		if let Some(message) = self.outbound.pop_front() {
			self.pending_outbound += 1;
			return Poll::Ready(ConnectionHandlerEvent::OutboundSubstreamRequest {
				protocol: SubstreamProtocol::new(message, ()),
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
			ConnectionEvent::FullyNegotiatedInbound(FullyNegotiatedInbound { protocol, .. }) => {
				self.pending_events.push_back(Event::Received { message: protocol });
			},
			ConnectionEvent::FullyNegotiatedOutbound(FullyNegotiatedOutbound { protocol, .. }) => {
				self.pending_outbound -= 1;
				if let Some(message) = protocol {
					self.pending_events.push_back(Event::Sent { message });
				}
			},
			ConnectionEvent::DialUpgradeError(dial_upgrade_error) => self.on_dial_upgrade_error(dial_upgrade_error),
			ConnectionEvent::ListenUpgradeError(listen_upgrade_error) => {
				self.on_listen_upgrade_error(listen_upgrade_error)
			},
			_ => {},
		}
	}
}
