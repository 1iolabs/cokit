use crate::{
	services::connections::{
		ConnectionAction, ConnectionMessage, PeerConnectionClosedAction, PeerConnectionEstablishedAction,
	},
	NetworkTask,
};
use co_actor::ActorHandle;
use libp2p::{
	swarm::{NetworkBehaviour, SwarmEvent},
	Swarm,
};
use std::time::Instant;

/// Monitor connnections.
#[derive(Debug)]
pub struct ConnectionsNetworkTask {
	handle: ActorHandle<ConnectionMessage>,
}
impl ConnectionsNetworkTask {
	pub fn new(handle: ActorHandle<ConnectionMessage>) -> Self {
		Self { handle }
	}
}
impl<B, C> NetworkTask<B, C> for ConnectionsNetworkTask
where
	B: NetworkBehaviour,
{
	fn execute(&mut self, _swarm: &mut Swarm<B>, _context: &mut C) {}

	/// Handle swarm events.
	/// Events can be consumed by this handler or forwarded to next handler.
	fn on_swarm_event(
		&mut self,
		_swarm: &mut Swarm<B>,
		_context: &mut C,
		event: SwarmEvent<B::ToSwarm>,
	) -> Option<SwarmEvent<B::ToSwarm>> {
		match &event {
			SwarmEvent::ConnectionEstablished {
				peer_id,
				connection_id: _,
				endpoint: _,
				num_established,
				concurrent_dial_errors: _,
				established_in: _,
			} if num_established.get() == 1 => {
				self.handle
					.dispatch(ConnectionAction::PeerConnectionEstablished(PeerConnectionEstablishedAction {
						peer_id: *peer_id,
						time: Instant::now(),
					}))
					.ok();
			},
			SwarmEvent::ConnectionClosed { peer_id, connection_id: _, endpoint: _, num_established, cause: _ }
				if *num_established == 0 =>
			{
				self.handle
					.dispatch(ConnectionAction::PeerConnectionClosed(PeerConnectionClosedAction {
						peer_id: *peer_id,
						time: Instant::now(),
					}))
					.ok();
			},
			_ => {},
		}
		Some(event)
	}

	/// Test if the task is complete and can be removed from the queue.
	/// This will be called only after execute has been called.
	fn is_complete(&mut self) -> bool {
		self.handle.is_closed()
	}
}
