// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{
	backoff,
	network::{Behaviour, NetworkEvent},
	types::network_task::{NetworkTask, NetworkTaskState},
};
use co_actor::time::Instant;
use libp2p::{core::transport::ListenerId, swarm::SwarmEvent, Multiaddr, Swarm, TransportError};

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
	fatal: bool,
}
impl ListenTask {
	pub fn new(addr: Multiaddr) -> Self {
		Self { addr, listener_id: None, backoff_retry: 0, backoff_until: None, fatal: false }
	}
}
impl NetworkTask<Behaviour> for ListenTask {
	fn execute(&mut self, swarm: &mut Swarm<Behaviour>) {
		// remove our own stale listener (if any) before re-listening, so we never leak one
		if let Some(stale) = self.listener_id.take() {
			swarm.remove_listener(stale);
		}
		match swarm.listen_on(self.addr.clone()) {
			Ok(listener_id) => {
				tracing::trace!(?listener_id, addr = ?self.addr, "network-listen");
				self.listener_id = Some(listener_id);
			},
			Err(error) if is_unsupported_address_family(&error) => {
				// ip version disabled on this host: stop retrying this listener.
				tracing::info!(addr = ?self.addr, "network-listen-unsupported");
				self.fatal = true;
			},
			Err(error) => {
				// transient listen failure: back off and retry instead of spinning.
				tracing::trace!(?error, addr = ?self.addr, "network-listen-failed");
				self.backoff_retry += 1;
				self.backoff_until = Some(Instant::now() + backoff(self.backoff_retry));
			},
		}
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
		self.fatal
	}

	fn task_state(&mut self) -> NetworkTaskState {
		if self.fatal {
			return NetworkTaskState::Complete;
		}
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

/// whether `error` is an "address family not supported" bind failure — the host has this ip
/// version disabled, so re-listening will never succeed. eafnosupport: apple/bsd 47,
/// linux/android 97, windows wsaeafnosupport 10047.
fn is_unsupported_address_family(error: &TransportError<std::io::Error>) -> bool {
	matches!(
		error,
		TransportError::Other(io_error)
			if matches!(io_error.raw_os_error(), Some(47) | Some(97) | Some(10047))
	)
}

#[cfg(test)]
mod tests {
	use super::is_unsupported_address_family;
	use libp2p::TransportError;
	use std::io;

	#[test]
	fn detects_unsupported_address_family() {
		// eafnosupport: apple/bsd 47, linux/android 97, windows wsaeafnosupport 10047.
		for code in [47, 97, 10047] {
			let error = TransportError::Other(io::Error::from_raw_os_error(code));
			assert!(is_unsupported_address_family(&error));
		}
	}

	#[test]
	fn ignores_other_errors() {
		let error = TransportError::Other(io::Error::from_raw_os_error(1));
		assert!(!is_unsupported_address_family(&error));
		let error: TransportError<io::Error> =
			TransportError::MultiaddrNotSupported("/ip4/0.0.0.0/udp/0/quic-v1".parse().unwrap());
		assert!(!is_unsupported_address_family(&error));
	}
}
