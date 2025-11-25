use crate::{
	backoff,
	network::{Behaviour, Context, NetworkEvent},
	types::network_task::NetworkTask,
};
use libp2p::{core::transport::ListenerId, swarm::SwarmEvent, Multiaddr, Swarm};
use multiaddr::Protocol;
use tokio::time::Instant;

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
		swarm: &mut Swarm<Behaviour>,
		context: &mut Context,
		event: SwarmEvent<NetworkEvent>,
	) -> Option<SwarmEvent<NetworkEvent>> {
		// event
		match &event {
			// SwarmEvent::ListenerError { listener_id, error } => {},
			SwarmEvent::ListenerClosed { listener_id, .. } => {
				if Some(listener_id) == self.listener_id.as_ref() {
					self.listener_id = None;
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

		// listen
		//  TODO: move this to tick or something as currently we only retry on some events
		if self.listener_id.is_none() {
			if match self.backoff_until {
				Some(until) if until < Instant::now() => true,
				None => true,
				_ => false,
			} {
				// listen
				self.execute(swarm, context);

				// adjust backoff
				self.backoff_retry += 1;
				self.backoff_until = Some(Instant::now() + backoff(self.backoff_retry));
			}
		}

		Some(event)
	}

	fn is_complete(&mut self) -> bool {
		false
	}
}
