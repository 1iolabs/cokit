// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{
	network::{Behaviour, NetworkEvent},
	services::connections::{
		action::{
			ConnectionAction, PeerConnectionClosedAction, PeerConnectionEstablishedAction, PeerHolePunchedAction,
		},
		ConnectionDirection, ConnectionEndpoint, ConnectionMessage,
	},
	types::network_task::NetworkTask,
};
use co_actor::{time::Instant, ActorHandle};
use libp2p::{core::ConnectedPoint, swarm::SwarmEvent, Swarm};

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
impl NetworkTask<Behaviour> for ConnectionsNetworkTask {
	fn execute(&mut self, _swarm: &mut Swarm<Behaviour>) {}

	/// Handle swarm events.
	/// Events can be consumed by this handler or forwarded to next handler.
	fn on_swarm_event(
		&mut self,
		_swarm: &mut Swarm<Behaviour>,
		event: SwarmEvent<NetworkEvent>,
	) -> Option<SwarmEvent<NetworkEvent>> {
		match &event {
			SwarmEvent::ConnectionEstablished { peer_id, connection_id, endpoint, .. } => {
				let (local, direction) = match endpoint {
					ConnectedPoint::Listener { local_addr, .. } => {
						(Some(local_addr.clone()), ConnectionDirection::Incoming)
					},
					ConnectedPoint::Dialer { .. } => (None, ConnectionDirection::Outgoing),
				};
				self.handle
					.dispatch(ConnectionAction::PeerConnectionEstablished(PeerConnectionEstablishedAction {
						peer_id: *peer_id,
						connection_id: *connection_id,
						endpoint: ConnectionEndpoint {
							remote: endpoint.get_remote_address().clone(),
							local,
							direction,
							hole_punched: false,
						},
						time: Instant::now(),
					}))
					.ok();
			},
			SwarmEvent::ConnectionClosed { peer_id, connection_id, .. } => {
				self.handle
					.dispatch(ConnectionAction::PeerConnectionClosed(PeerConnectionClosedAction {
						peer_id: *peer_id,
						connection_id: *connection_id,
						time: Instant::now(),
					}))
					.ok();
			},
			// new direct connection via a successful hole-punch by libp2p-dcutr
			SwarmEvent::Behaviour(NetworkEvent::Dcutr(dcutr_event)) => {
				if let Ok(connection_id) = &dcutr_event.result {
					self.handle
						.dispatch(ConnectionAction::PeerHolePunched(PeerHolePunchedAction {
							peer_id: dcutr_event.remote_peer_id,
							connection_id: *connection_id,
						}))
						.ok();
				}
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
