// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{
	backoff,
	network::{Behaviour, Context, NetworkEvent},
	types::network_task::{NetworkTask, NetworkTaskState},
};
use libp2p::{core::transport::ListenerId, swarm::SwarmEvent, Multiaddr, Swarm};
use multiaddr::Protocol;
use std::time::Instant;

/// Try to listen to a relay when we are behind NAT.
#[derive(Debug)]
pub struct RelayListenTask {
	relay: Multiaddr,
	listener_id: Option<ListenerId>,
	backoff_retry: u32,
	backoff_until: Option<Instant>,
}
impl RelayListenTask {
	pub fn new(relay: Multiaddr) -> Self {
		Self { relay, listener_id: None, backoff_retry: 0, backoff_until: None }
	}
}
impl NetworkTask<Behaviour, Context> for RelayListenTask {
	fn execute(&mut self, swarm: &mut Swarm<Behaviour>, _context: &mut Context) {
		let listen_addr = self.relay.clone().with(Protocol::P2pCircuit);
		let result = swarm.listen_on(listen_addr.clone());
		tracing::trace!(?result, ?listen_addr, "network-relay-listen");
		self.listener_id = result.ok();
	}

	fn on_swarm_event(
		&mut self,
		_swarm: &mut Swarm<Behaviour>,
		_context: &mut Context,
		event: SwarmEvent<NetworkEvent>,
	) -> Option<SwarmEvent<NetworkEvent>> {
		// event
		match &event {
			// SwarmEvent::ListenerError { listener_id, error } => {},
			SwarmEvent::ListenerClosed { listener_id, .. } => {
				if Some(listener_id) == self.listener_id.as_ref() {
					self.listener_id = None;
					self.backoff_retry += 1;
					self.backoff_until = Some(Instant::now() + backoff(self.backoff_retry));
				}
			},
			SwarmEvent::NewListenAddr { listener_id, .. } => {
				if Some(listener_id) == self.listener_id.as_ref() {
					self.backoff_retry = 0;
					self.backoff_until = None;
				}
			},
			_ => {},
		}
		Some(event)
	}

	fn is_complete(&mut self) -> bool {
		false
	}

	fn task_state(&mut self) -> NetworkTaskState {
		match self.listener_id {
			Some(_) => NetworkTaskState::Waiting,
			None => match self.backoff_until {
				Some(until) => {
					if until < Instant::now() {
						NetworkTaskState::Pending
					} else {
						NetworkTaskState::Delayed(until)
					}
				},
				None => NetworkTaskState::Pending,
			},
		}
	}
}
