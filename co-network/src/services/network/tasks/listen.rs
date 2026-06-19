// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{
	backoff,
	network::{Behaviour, NetworkEvent},
	types::network_task::{NetworkTask, NetworkTaskState},
};
use co_actor::time::Instant;
use libp2p::{core::transport::ListenerId, swarm::SwarmEvent, Multiaddr, Swarm};

/// Listen on an address and keep the listener alive.
///
/// Owns its `ListenerId` and re-`listen_on`s with backoff when the listener closes. Used for
/// both the main listen address and relay circuit addresses — the caller builds the address
/// (e.g. appends `/p2p-circuit` for a relay), so the task stays address-agnostic.
#[derive(Debug)]
pub struct ListenTask {
	addr: Multiaddr,
	listener_id: Option<ListenerId>,
	backoff_retry: u32,
	backoff_until: Option<Instant>,
}
impl ListenTask {
	pub fn new(addr: Multiaddr) -> Self {
		Self { addr, listener_id: None, backoff_retry: 0, backoff_until: None }
	}
}
impl NetworkTask<Behaviour> for ListenTask {
	fn execute(&mut self, swarm: &mut Swarm<Behaviour>) {
		// remove our own stale listener (if any) before re-listening, so we never leak one
		if let Some(stale) = self.listener_id.take() {
			swarm.remove_listener(stale);
		}
		let result = swarm.listen_on(self.addr.clone());
		tracing::trace!(?result, addr = ?self.addr, "network-listen");
		self.listener_id = result.ok();
	}

	fn on_swarm_event(
		&mut self,
		_swarm: &mut Swarm<Behaviour>,
		event: SwarmEvent<NetworkEvent>,
	) -> Option<SwarmEvent<NetworkEvent>> {
		match &event {
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
